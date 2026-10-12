package io.github.cumsauce.foley;

import com.mojang.brigadier.arguments.FloatArgumentType;
import io.github.cumsauce.foley.audio.GeneratedSounds;
import io.github.cumsauce.foley.audio.RainSound;
import io.github.cumsauce.foley.natives.NativeBridge;
import io.github.cumsauce.foley.natives.NativeLoader;
import java.nio.file.Path;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.command.v2.ClientCommandRegistrationCallback;
import net.fabricmc.fabric.api.client.command.v2.ClientCommands;
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
	private static @Nullable RainSound rain;

	public static Path configDir() {
		return FabricLoader.getInstance().getConfigDir().resolve("foley");
	}

	public static @Nullable NativeBridge bridge() {
		return bridge;
	}

	/** The rain playing, or null. */
	public static @Nullable RainSound rain() {
		return rain;
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
		registerCommands(loaded);
	}

	/** Starts rain on water at {@code rate} drops a second, or stops it at 0. */
	public static void setRain(float rate) {
		Minecraft minecraft = Minecraft.getInstance();
		if (rain != null) minecraft.getSoundManager().stop(rain);
		rain = null;
		if (rate <= 0 || bridge == null) return;
		rain = new RainSound(bridge, rate);
		minecraft.getSoundManager().play(rain);
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
								.then(ClientCommands.argument("drops", FloatArgumentType.floatArg(0, 5000))
										.executes(c -> {
											setRain(FloatArgumentType.getFloat(c, "drops"));
											c.getSource().sendFeedback(Component.literal("Rain: "
													+ FloatArgumentType.getFloat(c, "drops") + " drops a second"));
											return 1;
										})))
						.then(ClientCommands.literal("drip").executes(c -> {
							var at = c.getSource().getPosition();
							drip(at.x, at.y, at.z);
							return 1;
						}))));
	}
}
