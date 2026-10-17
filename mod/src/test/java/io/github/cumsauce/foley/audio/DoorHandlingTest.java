package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;

import io.github.cumsauce.foley.api.DoorSoundHandler;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;

/** S1: the seam that decides between the game's door sound, Foley's contact tap and an addon's handler. */
class DoorHandlingTest {
	@AfterEach
	void clearHandler() {
		DoorHandling.handler(null);
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
	void theApiExposesTheHandler() {
		FoleyApiImpl api = new FoleyApiImpl("mymod");
		DoorSoundHandler handler = (event, x, y, z, iron) -> false;
		api.doorHandler(handler);
		assertSame(handler, api.doorHandler());
		api.doorHandler(null);
		assertNull(api.doorHandler(), "the facade clears it too");
	}
}
