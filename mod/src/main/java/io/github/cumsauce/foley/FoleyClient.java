package io.github.cumsauce.foley;

import com.mojang.brigadier.arguments.FloatArgumentType;
import io.github.cumsauce.foley.audio.GeneratedSounds;
import io.github.cumsauce.foley.audio.RainController;
import io.github.cumsauce.foley.natives.NativeBridge;
import io.github.cumsauce.foley.natives.NativeLoader;
import java.nio.file.Path;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.command.v2.ClientCommandRegistrationCallback;
import net.fabricmc.fabric.api.client.command.v2.ClientCommands;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.loader.api.FabricLoader;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.sounds.SoundSource;
import org.jspecify.annotations.Nullable;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public final class FoleyClient implements ClientModInitializer {
	public static final Logger LOGGER = LoggerFactory.getLogger("foley");
	private static final SoundEvent DRIP = SoundEvent.createVariableRangeEvent(
			Identifier.fromNamespaceAndPath("foley", "drip"));

	private static @Nullable NativeBridge bridge;
	private static @Nullable RainController rain;
	private static final java.util.concurrent.atomic.AtomicInteger replaced = new java.util.concurrent.atomic.AtomicInteger();

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

	/** Counts a game sound Foley replaced (skipped). */
	public static void countReplaced() {
		replaced.incrementAndGet();
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
		GeneratedSounds.use(loaded);
		LOGGER.info("Started Foley's native synthesis (ABI {})", NativeBridge.ABI_VERSION);
		RainController controller = new RainController(loaded);
		rain = controller;
		ClientTickEvents.END_CLIENT_TICK.register(controller::tick);
		registerCommands(loaded);
	}

	/** A generated drip at the player's feet, a different one each time. */
	public static SimpleSoundInstance drip(double x, double y, double z) {
		SimpleSoundInstance drip = new SimpleSoundInstance(DRIP, SoundSource.AMBIENT, 1f, 1f,
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
						.then(ClientCommands.literal("scene").executes(c -> {
							c.getSource().sendFeedback(Component.literal(rain == null ? "Foley is off"
									: rain.scene().toString()));
							return 1;
						}))
						.then(ClientCommands.literal("drip").executes(c -> {
							var at = c.getSource().getPosition();
							drip(at.x, at.y, at.z);
							return 1;
						}))));
	}
}
