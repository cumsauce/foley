package io.github.cumsauce.foley.gametest;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.audio.RainSound;
import io.github.cumsauce.foley.natives.NativeBridge;
import net.fabricmc.fabric.api.client.gametest.v1.FabricClientGameTest;
import net.fabricmc.fabric.api.client.gametest.v1.context.ClientGameTestContext;
import net.fabricmc.fabric.api.client.gametest.v1.context.TestSingleplayerContext;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;

/**
 * Starts the real client and checks M0: the native synthesis loaded, a synthesized stream (rain on water) plays
 * through the game's sound engine and keeps being rendered, and a generated one-shot (a drip) plays from the game's
 * static sounds. CI has no sound card, so it runs with OpenAL Soft's null backend.
 */
@SuppressWarnings("UnstableApiUsage")
public class FoleyClientGameTest implements FabricClientGameTest {
	@Override
	public void runTest(ClientGameTestContext context) {
		if (FoleyClient.bridge() == null) throw new AssertionError("the native synthesis didn't load");
		try (TestSingleplayerContext singleplayer = context.worldBuilder().create()) {
			singleplayer.getConnection().waitForChunksRender();

			// The stream: the game reads it buffer after buffer.
			context.runOnClient(mc -> FoleyClient.setRain(400));
			try {
				context.waitFor(mc -> {
					RainSound rain = FoleyClient.rain();
					return rain != null && mc.getSoundManager().isActive(rain)
							&& rain.rendered() > NativeBridge.SAMPLE_RATE;
				}, 10 * 20);
			} catch (RuntimeException | AssertionError e) {
				RainSound rain = FoleyClient.rain();
				throw new AssertionError("the rain stream isn't playing: rendered "
						+ (rain == null ? "nothing" : rain.rendered() + " samples"), e);
			}
			context.runOnClient(mc -> FoleyClient.setRain(0));

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
