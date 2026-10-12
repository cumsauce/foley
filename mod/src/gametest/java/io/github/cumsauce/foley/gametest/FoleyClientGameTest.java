package io.github.cumsauce.foley.gametest;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.audio.Surfaces;
import io.github.cumsauce.foley.natives.NativeBridge;
import net.fabricmc.fabric.api.client.gametest.v1.FabricClientGameTest;
import net.fabricmc.fabric.api.client.gametest.v1.context.ClientGameTestContext;
import net.fabricmc.fabric.api.client.gametest.v1.context.TestSingleplayerContext;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;

/**
 * Starts the real client and checks: the native synthesis loaded; when it rains, Foley's rain (a synthesized stream)
 * plays through the game's sound engine, follows what it lands on and any roof, and replaces the game's rain sounds
 * (M1); and a generated one-shot (a drip) plays from the game's static sounds (M0). CI has no sound card, so it runs with OpenAL Soft's null backend.
 */
@SuppressWarnings("UnstableApiUsage")
public class FoleyClientGameTest implements FabricClientGameTest {
	@Override
	public void runTest(ClientGameTestContext context) {
		if (FoleyClient.bridge() == null) throw new AssertionError("the native synthesis didn't load");
		try (TestSingleplayerContext singleplayer = context.worldBuilder().create()) {
			singleplayer.getConnection().waitForChunksRender();

			// The rain: when it rains, Foley's starts, follows what it lands on (a superflat's grass is soft), keeps
			// being rendered, and the game's own rain sounds are skipped.
			singleplayer.getServer().runCommand("weather rain");
			try {
				context.waitFor(mc -> {
					var rain = FoleyClient.rain();
					return rain != null && rain.sound() != null && mc.getSoundManager().isActive(rain.sound())
							&& rain.sound().rendered() > NativeBridge.SAMPLE_RATE && rain.scene().rate() > 5
							&& rain.scene().surfaces()[Surfaces.SOFT] > 0.5f;
				}, 20 * 20);
			} catch (RuntimeException | AssertionError e) {
				var rain = FoleyClient.rain();
				throw new AssertionError("Foley's rain isn't playing: " + (rain == null ? "off" : rain.scene() + ", sound "
						+ rain.sound()), e);
			}
			try {
				context.waitFor(mc -> FoleyClient.replaced() > 0, 20 * 20);
			} catch (RuntimeException | AssertionError e) {
				throw new AssertionError("the game's rain sounds were never replaced", e);
			}
			// Under a wooden roof, it drums on the roof.
			singleplayer.getServer().runCommand("fill ~-3 ~3 ~-3 ~3 ~3 ~3 oak_planks");
			try {
				context.waitFor(mc -> FoleyClient.rain().scene().roof() == Surfaces.WOOD, 10 * 20);
			} catch (RuntimeException | AssertionError e) {
				throw new AssertionError("no roof: " + FoleyClient.rain().scene(), e);
			}
			singleplayer.getServer().runCommand("weather clear");

			// The one-shot: a generated buffer, played like any sound.
			SimpleSoundInstance drip = context.computeOnClient(mc -> FoleyClient.drip(
					mc.player.getX(), mc.player.getEyeY(), mc.player.getZ()));
			try {
				context.waitFor(mc -> mc.getSoundManager().isActive(drip), 5 * 20);
			} catch (RuntimeException | AssertionError e) {
				throw new AssertionError("the generated drip never played", e);
			}
		}
	}
}
