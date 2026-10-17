package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertTrue;

import io.github.cumsauce.foley.api.DoorSoundHandler;
import io.github.cumsauce.foley.api.SoundOverrideRegistration;
import java.util.concurrent.atomic.AtomicInteger;
import net.minecraft.resources.Identifier;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

/**
 * S2/S3: the generic vanilla-sound override seam and the door sugar built on top of it.
 *
 * <p>Every test starts and ends with an empty registry, so registration order is the test's own and nothing leaks
 * between tests. No game instance is needed: dispatch is a pure decision over predicates and handlers.
 */
class DoorHandlingTest {
	// The ids the game's SoundEvents use, written here so the test needs no registry or client to boot.
	private static final Identifier WOODEN_DOOR_OPEN = Identifier.fromNamespaceAndPath("minecraft",
			"block.wooden_door.open");
	private static final Identifier WOODEN_DOOR_CLOSE = Identifier.fromNamespaceAndPath("minecraft",
			"block.wooden_door.close");
	private static final Identifier IRON_TRAPDOOR_CLOSE = Identifier.fromNamespaceAndPath("minecraft",
			"block.iron_trapdoor.close");
	private static final Identifier NOT_A_DOOR = Identifier.fromNamespaceAndPath("minecraft", "block.stone.break");

	@BeforeEach
	void reset() {
		DoorHandling.clearOverrides();
	}

	@AfterEach
	void cleanUp() {
		DoorHandling.clearOverrides();
	}

	@Test
	void noOverrideFallsThroughToNone() {
		assertNull(DoorHandling.handler(), "the built-in tap is the default");
		assertEquals(DoorHandling.Action.NONE, DoorHandling.dispatch(WOODEN_DOOR_OPEN, 0, 0, 0),
				"no override registered: Foley's own behaviour applies");
		assertEquals(DoorHandling.Action.NONE, DoorHandling.dispatch(NOT_A_DOOR, 0, 0, 0),
				"a non-door event falls through too");
	}

	@Test
	void registerReturnsAHandleThatUnregisters() {
		SoundOverrideRegistration handled = DoorHandling.register(DoorHandling::isDoorEvent,
				(event, x, y, z) -> true);
		assertEquals(DoorHandling.Action.OVERRIDE_HANDLED, DoorHandling.dispatch(WOODEN_DOOR_OPEN, 1, 2, 3));
		handled.unregister();
		assertEquals(DoorHandling.Action.NONE, DoorHandling.dispatch(WOODEN_DOOR_OPEN, 1, 2, 3),
				"the handle removed the override");
		assertDoesNotThrow(handled::unregister, "unregistering twice is harmless");

		SoundOverrideRegistration declined = DoorHandling.register(event -> true, (event, x, y, z) -> false);
		assertEquals(DoorHandling.Action.OVERRIDE_VANILLA, DoorHandling.dispatch(WOODEN_DOOR_CLOSE, 1, 2, 3),
				"a declining override yields the vanilla sound");
		declined.unregister();
		assertEquals(DoorHandling.Action.NONE, DoorHandling.dispatch(WOODEN_DOOR_CLOSE, 1, 2, 3));
	}

	@Test
	void firstMatchingOverrideInReverseOrderWins() {
		AtomicInteger firstCalls = new AtomicInteger();
		AtomicInteger secondCalls = new AtomicInteger();
		DoorHandling.register(DoorHandling::isDoorEvent, (event, x, y, z) -> {
			firstCalls.incrementAndGet();
			return true;
		});
		DoorHandling.register(event -> true, (event, x, y, z) -> {
			secondCalls.incrementAndGet();
			return false;
		});

		assertEquals(DoorHandling.Action.OVERRIDE_VANILLA, DoorHandling.dispatch(WOODEN_DOOR_OPEN, 4, 5, 6),
				"the later registration decides, even though both predicates match");
		assertEquals(0, firstCalls.get(), "the earlier override was never consulted");
		assertEquals(1, secondCalls.get());
	}

	@Test
	void overrideReturningFalseYieldsVanilla() {
		DoorHandling.register(DoorHandling::isDoorEvent, (event, x, y, z) -> false);
		assertEquals(DoorHandling.Action.OVERRIDE_VANILLA, DoorHandling.dispatch(IRON_TRAPDOOR_CLOSE, 0, 0, 0));
	}

	@Test
	void overrideReturningTrueYieldsHandled() {
		DoorHandling.register(DoorHandling::isDoorEvent, (event, x, y, z) -> true);
		assertEquals(DoorHandling.Action.OVERRIDE_HANDLED, DoorHandling.dispatch(IRON_TRAPDOOR_CLOSE, 0, 0, 0));
	}

	@Test
	void clearRemovesAllOverrides() {
		DoorHandling.register(DoorHandling::isDoorEvent, (event, x, y, z) -> true);
		DoorHandling.register(event -> true, (event, x, y, z) -> false);
		DoorHandling.clearOverrides();

		assertEquals(DoorHandling.Action.NONE, DoorHandling.dispatch(WOODEN_DOOR_OPEN, 0, 0, 0),
				"the registry is empty again");
		assertEquals(DoorHandling.Action.NONE, DoorHandling.dispatch(NOT_A_DOOR, 0, 0, 0));
	}

	@Test
	void doorSugarRegistersAndDispatches() {
		AtomicInteger calls = new AtomicInteger();
		boolean[] ironSeen = new boolean[1];
		DoorSoundHandler handler = (event, x, y, z, iron) -> {
			calls.incrementAndGet();
			ironSeen[0] = iron;
			return true;
		};

		DoorHandling.handler(handler);
		assertSame(handler, DoorHandling.handler());

		assertEquals(DoorHandling.Action.OVERRIDE_HANDLED, DoorHandling.dispatch(IRON_TRAPDOOR_CLOSE, 0, 0, 0));
		assertTrue(ironSeen[0], "the iron flag is derived from the event id");

		assertEquals(DoorHandling.Action.OVERRIDE_HANDLED, DoorHandling.dispatch(WOODEN_DOOR_OPEN, 0, 0, 0));
		assertFalse(ironSeen[0], "a wooden event is not iron");
		assertEquals(2, calls.get());

		DoorHandling.handler(null);
		assertNull(DoorHandling.handler(), "null restores the built-in tap");
		assertEquals(DoorHandling.Action.NONE, DoorHandling.dispatch(WOODEN_DOOR_OPEN, 0, 0, 0),
				"the door sugar was unregistered");
	}

	@Test
	void nonDoorEventDoesNotTriggerDoorSugar() {
		AtomicInteger calls = new AtomicInteger();
		DoorHandling.handler((event, x, y, z, iron) -> {
			calls.incrementAndGet();
			return true;
		});

		assertEquals(DoorHandling.Action.NONE, DoorHandling.dispatch(NOT_A_DOOR, 0, 0, 0));
		assertEquals(0, calls.get(), "the door handler never saw a non-door event");
	}

	@Test
	void noHandlerKeepsFoleysTap() {
		assertNull(DoorHandling.handler(), "the built-in tap is the default");
		assertEquals(DoorHandling.Action.CONTACT_TAP, DoorHandling.action(false, true, false, false));
		assertEquals(DoorHandling.Action.PLAY_VANILLA, DoorHandling.action(false, false, false, false),
				"doors switched off: the game's sound plays");
	}

	@Test
	void handlerTakesOverFromTheTap() {
		assertEquals(DoorHandling.Action.ADDON, DoorHandling.action(false, true, true, true));
		assertEquals(DoorHandling.Action.PLAY_VANILLA, DoorHandling.action(false, true, true, false),
				"unhandled: fall back to the game's sound");
		assertEquals(DoorHandling.Action.ADDON, DoorHandling.action(false, false, true, true),
				"an addon owns doors even when Foley's own doors setting is off");
	}

	@Test
	void bypassWinsOverTheHandler() {
		assertEquals(DoorHandling.Action.PLAY_VANILLA, DoorHandling.action(true, true, true, true));
		assertEquals(DoorHandling.Action.PLAY_VANILLA, DoorHandling.action(true, true, false, false));
	}

	@Test
	void handlerCanBeRegisteredAndCleared() {
		DoorSoundHandler handler = (event, x, y, z, iron) -> true;
		DoorHandling.handler(handler);
		assertSame(handler, DoorHandling.handler());
		DoorHandling.handler(null);
		assertNull(DoorHandling.handler(), "null restores the built-in tap");
	}

	@Test
	void theApiFacadeExposesTheHandler() {
		FoleyApiImpl api = new FoleyApiImpl("mymod");
		DoorSoundHandler handler = (event, x, y, z, iron) -> false;
		api.doorHandler(handler);
		assertSame(handler, api.doorHandler());
		api.doorHandler(null);
		assertNull(api.doorHandler(), "the facade clears it too");
	}

	@Test
	void soundOverridesRegistryMatchesTheStatics() {
		SoundOverrideRegistration registration = DoorHandling.soundOverrides()
				.register(DoorHandling::isDoorEvent, (event, x, y, z) -> true);
		assertEquals(DoorHandling.Action.OVERRIDE_HANDLED, DoorHandling.dispatch(WOODEN_DOOR_CLOSE, 0, 0, 0));
		registration.unregister();
		assertEquals(DoorHandling.Action.NONE, DoorHandling.dispatch(WOODEN_DOOR_CLOSE, 0, 0, 0));

		DoorHandling.soundOverrides().register(DoorHandling::isDoorEvent, (event, x, y, z) -> true);
		DoorHandling.soundOverrides().clear();
		assertEquals(DoorHandling.Action.NONE, DoorHandling.dispatch(WOODEN_DOOR_CLOSE, 0, 0, 0),
				"the registry's clear empties the same list");
	}

	@Test
	void nonDoorEventHonoursAnOverride() {
		Identifier rain = Identifier.fromNamespaceAndPath("minecraft", "weather.rain");
		Identifier chest = Identifier.fromNamespaceAndPath("minecraft", "block.chest.open");
		DoorHandling.register(id -> id.equals(rain), (event, x, y, z) -> true);
		assertEquals(DoorHandling.Action.OVERRIDE_HANDLED, DoorHandling.dispatch(rain, 4, 5, 6));
		assertEquals(DoorHandling.Action.NONE, DoorHandling.dispatch(chest, 4, 5, 6),
				"an unrelated event is not matched");
		DoorHandling.clearOverrides();
		assertEquals(DoorHandling.Action.NONE, DoorHandling.dispatch(rain, 4, 5, 6));
	}

	@Test
	void decliningOverrideYieldsVanillaForAnyFamily() {
		Identifier thunder = Identifier.fromNamespaceAndPath("minecraft", "entity.lightning_bolt.thunder");
		DoorHandling.register(id -> id.equals(thunder), (event, x, y, z) -> false);
		assertEquals(DoorHandling.Action.OVERRIDE_VANILLA, DoorHandling.dispatch(thunder, 0, 0, 0));
	}

}
