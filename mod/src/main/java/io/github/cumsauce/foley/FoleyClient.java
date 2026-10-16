package io.github.cumsauce.foley;

import com.mojang.brigadier.arguments.FloatArgumentType;
import io.github.cumsauce.foley.audio.ActiveStructures;
import io.github.cumsauce.foley.audio.AddonSounds;
import io.github.cumsauce.foley.audio.Breathing;
import io.github.cumsauce.foley.audio.GeneratedSounds;
import io.github.cumsauce.foley.audio.MobBrushes;
import io.github.cumsauce.foley.audio.PeacefulBreathing;
import io.github.cumsauce.foley.audio.Settling;
import io.github.cumsauce.foley.config.FoleyConfig;
import io.github.cumsauce.foley.ui.BypassKey;
import io.github.cumsauce.foley.ui.SoundOptionsButton;
import io.github.cumsauce.foley.audio.Life;
import io.github.cumsauce.foley.audio.PlacedBeds;
import io.github.cumsauce.foley.audio.Replacements;
import io.github.cumsauce.foley.audio.ThunderController;
import io.github.cumsauce.foley.audio.RainController;
import io.github.cumsauce.foley.audio.WindController;
import io.github.cumsauce.foley.natives.NativeBridge;
import io.github.cumsauce.foley.natives.NativeLoader;
import java.nio.file.Path;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.command.v2.ClientCommandRegistrationCallback;
import net.fabricmc.fabric.api.client.command.v2.ClientCommands;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientEntityEvents;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.loader.api.FabricLoader;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.network.chat.Component;
import net.minecraft.sounds.SoundSource;
import org.jspecify.annotations.Nullable;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public final class FoleyClient implements ClientModInitializer {
	public static final Logger LOGGER = LoggerFactory.getLogger("foley");

	private static @Nullable NativeBridge bridge;
	private static @Nullable RainController rain;
	private static @Nullable WindController wind;
	private static @Nullable PlacedBeds placed;
	private static @Nullable ThunderController thunder;
	private static @Nullable Life life;
	private static @Nullable Breathing breathing;
	private static @Nullable ActiveStructures activeStructures;
	private static @Nullable Settling settling;
	private static @Nullable MobBrushes mobBrushes;
	private static @Nullable PeacefulBreathing peacefulBreathing;
	private static volatile FoleyConfig config = FoleyConfig.DEFAULT;
	private static volatile boolean bypassed;
	private static final java.util.concurrent.atomic.AtomicInteger replaced = new java.util.concurrent.atomic.AtomicInteger();
	private static final java.util.concurrent.atomic.AtomicInteger layered = new java.util.concurrent.atomic.AtomicInteger();

	public static Path configDir() {
		return FabricLoader.getInstance().getConfigDir().resolve("foley");
	}

	public static @Nullable NativeBridge bridge() {
		return bridge;
	}

	/** What keeps Foley's rain going, or null if Foley is off. */
	public static @Nullable RainController rain() {
		return rain;
	}

	/** What keeps Foley's wind blowing, or null if Foley is off. */
	public static @Nullable WindController wind() {
		return wind;
	}

	/** What keeps the water, fire and lava going, or null if Foley is off. */
	public static @Nullable PlacedBeds placed() {
		return placed;
	}

	/** M21: minecarts, doors, chests, redstone, boats, sculk. */
	public static @Nullable ActiveStructures activeStructures() {
		return activeStructures;
	}

	/** M22: the settle tick under footsteps and block placement. */
	public static @Nullable Settling settling() {
		return settling;
	}

	/** M22: sheep, spiders and chickens brushing through plants. */
	public static @Nullable MobBrushes mobBrushes() {
		return mobBrushes;
	}

	/** M22: cows and sheep breathing quietly when Foley voices them. */
	public static @Nullable PeacefulBreathing peacefulBreathing() {
		return peacefulBreathing;
	}

	/** One-shots Foley may start in a tick; past it, the game's own sound plays alone. */
	private static final int ONE_SHOTS_PER_TICK = 6;
	private static int oneShots;

	/**
	 * Whether Foley may start one more one-shot this tick. A cap, so that many sounds at once (a crowd of mobs, a
	 * battle) never fill the game's pool of sound sources with Foley's layers and starve new sounds.
	 */
	public static boolean oneShot() {
		return oneShots++ < ONE_SHOTS_PER_TICK;
	}

	/** The settings in use. */
	public static FoleyConfig config() {
		return config;
	}

	/** Uses {@code next} from now on, and saves it. */
	public static void apply(FoleyConfig next) {
		config = next;
		next.save(configDir().resolve("foley.json"));
	}

	/** Whether the A/B key has switched Foley off for now. */
	public static boolean bypassed() {
		return bypassed;
	}

	public static void setBypassed(boolean bypassed) {
		FoleyClient.bypassed = bypassed;
	}

	/** Whether Foley should make the sound {@code setting} picks: not bypassed, audible, and switched on. */
	public static boolean wants(java.util.function.Predicate<FoleyConfig> setting) {
		FoleyConfig now = config;
		return !bypassed && now.volume() > 0 && setting.test(now);
	}

	/** Whether Earshot is installed to shape sounds (and so to fade placed beds with distance). */
	public static boolean earshot() {
		return EARSHOT;
	}

	private static final boolean EARSHOT = net.fabricmc.loader.api.FabricLoader.getInstance().isModLoaded("earshot");

	/** Foley's volume as a gain, 0 to 1. */
	public static float gain() {
		return config.gain();
	}

	/** What makes the thunder, or null if Foley is off. */
	public static @Nullable ThunderController thunder() {
		return thunder;
	}

	/** The birds, crickets and frogs, or null if Foley is off. */
	public static @Nullable Life life() {
		return life;
	}

	/** The zombies' breathing, or null if Foley is off. */
	public static @Nullable Breathing breathing() {
		return breathing;
	}

	/** Counts a game sound Foley replaced (skipped). */
	public static void countReplaced() {
		replaced.incrementAndGet();
	}

	/** Counts a strike or break Foley layered under a game sound. */
	public static void countLayered() {
		layered.incrementAndGet();
	}

	/** How many strikes and breaks Foley has layered. For the client gametest. */
	public static int layered() {
		return layered.get();
	}

	/** How many game sounds Foley has replaced. For the client gametest. */
	public static int replaced() {
		return replaced.get();
	}

	@Override
	public void onInitializeClient() {
		NativeBridge loaded;
		try {
			loaded = new NativeBridge(NativeLoader.load(configDir()));
		} catch (Exception | LinkageError e) {
			LOGGER.error("Foley is off: could not load its native synthesis", e);
			return;
		}
		if (loaded.abiVersion() != NativeBridge.ABI_VERSION) {
			LOGGER.error("Foley is off: native ABI {} does not match expected {}", loaded.abiVersion(),
					NativeBridge.ABI_VERSION);
			return;
		}
		bridge = loaded;
		config = FoleyConfig.load(configDir().resolve("foley.json"));
		SoundOptionsButton.register();
		BypassKey.register();
		GeneratedSounds.use(loaded);
		LOGGER.info("Started Foley's native synthesis (ABI {})", NativeBridge.ABI_VERSION);
		AddonSounds.start(loaded);
		RainController controller = new RainController(loaded);
		rain = controller;
		ClientTickEvents.END_CLIENT_TICK.register(controller::tick);
		WindController blowing = new WindController(loaded);
		wind = blowing;
		ClientTickEvents.END_CLIENT_TICK.register(blowing::tick);
		PlacedBeds beds = new PlacedBeds(loaded);
		placed = beds;
		ClientTickEvents.END_CLIENT_TICK.register(beds::tick);
		ThunderController rumbling = new ThunderController(loaded);
		thunder = rumbling;
		ClientTickEvents.END_CLIENT_TICK.register(rumbling::tick);
		ClientEntityEvents.ENTITY_LOAD.register((entity, level) -> rumbling.entityLoaded(entity));
		Life living = new Life();
		life = living;
		ClientTickEvents.END_CLIENT_TICK.register(living::tick);
		Breathing breaths = new Breathing();
		breathing = breaths;
		ClientTickEvents.END_CLIENT_TICK.register(breaths::tick);
		ClientTickEvents.END_CLIENT_TICK.register(new io.github.cumsauce.foley.audio.Brush()::tick);
		ActiveStructures structures = new ActiveStructures(loaded);
		activeStructures = structures;
		ClientTickEvents.END_CLIENT_TICK.register(structures::tick);
		Settling set = new Settling(loaded);
		settling = set;
		MobBrushes brushes = new MobBrushes();
		mobBrushes = brushes;
		ClientTickEvents.END_CLIENT_TICK.register(brushes::tick);
		PeacefulBreathing quiet = new PeacefulBreathing();
		peacefulBreathing = quiet;
		ClientTickEvents.END_CLIENT_TICK.register(quiet::tick);
		ClientTickEvents.START_CLIENT_TICK.register(minecraft -> oneShots = 0);
		registerCommands(loaded);
	}

	/** A generated drip at the player's feet, a different one each time. */
	public static SimpleSoundInstance drip(double x, double y, double z) {
		SimpleSoundInstance drip = new SimpleSoundInstance(Replacements.DRIP, SoundSource.AMBIENT, 1f, 1f,
				net.minecraft.client.resources.sounds.SoundInstance.createUnseededRandom(), x, y, z);
		Minecraft.getInstance().getSoundManager().play(drip);
		return drip;
	}

	private static void registerCommands(NativeBridge bridge) {
		ClientCommandRegistrationCallback.EVENT.register((dispatcher, registryAccess) -> dispatcher.register(
				ClientCommands.literal("foley")
						.then(ClientCommands.literal("rain")
								.then(ClientCommands.literal("auto").executes(c -> {
									if (rain != null) rain.force(null);
									c.getSource().sendFeedback(Component.literal("Rain follows the weather"));
									return 1;
								}))
								.then(ClientCommands.argument("mm_per_hour", FloatArgumentType.floatArg(0, 200))
										.executes(c -> {
											float rate = FloatArgumentType.getFloat(c, "mm_per_hour");
											if (rain != null) rain.force(rate);
											c.getSource().sendFeedback(Component.literal("Rain: " + rate + " mm/h"));
											return 1;
										})))
						.then(ClientCommands.literal("wind")
								.then(ClientCommands.literal("auto").executes(c -> {
									if (wind != null) wind.force(null);
									c.getSource().sendFeedback(Component.literal("Wind follows the weather"));
									return 1;
								}))
								.then(ClientCommands.argument("m_per_second", FloatArgumentType.floatArg(0, 60))
										.executes(c -> {
											float speed = FloatArgumentType.getFloat(c, "m_per_second");
											if (wind != null) wind.force(speed);
											c.getSource().sendFeedback(Component.literal("Wind: " + speed + " m/s"));
											return 1;
										})))
						.then(ClientCommands.literal("scene").executes(c -> {
							c.getSource().sendFeedback(Component.literal(rain == null || wind == null || placed == null
									? "Foley is off"
									: rain.scene() + "\n" + wind.scene() + "\n" + placed.around().water() + "\n"
											+ placed.around().fire() + "\n" + placed.around().lava()));
							return 1;
						}))
						.then(ClientCommands.literal("drip").executes(c -> {
							var at = c.getSource().getPosition();
							drip(at.x, at.y, at.z);
							return 1;
						}))));
	}
}
