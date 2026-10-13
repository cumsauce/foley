package io.github.cumsauce.foley.gametest;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import net.fabricmc.fabric.api.client.gametest.v1.FabricClientGameTest;
import net.fabricmc.fabric.api.client.gametest.v1.context.ClientGameTestContext;
import net.fabricmc.fabric.api.client.gametest.v1.context.TestSingleplayerContext;
import net.minecraft.sounds.SoundSource;

/**
 * Not a test: a busy scene to listen to, for finding what a mix is made of. Runs only with {@code FOLEY_SCENE=1} in
 * the environment, and is meant to be recorded through OpenAL Soft's wave writer ({@code ALSOFT_CONF} naming a config
 * with {@code drivers=wave}). Trees, flowing water, a campfire and a crowd of animals, villagers and husks, with
 * Earshot (if present) on High with convolution; then phases of {@value #PHASE_TICKS} ticks, each after a second of
 * silence and logged with its wall-clock start ("SCENE phase"): the game alone; with Earshot; with Foley's ambience;
 * its footsteps and strikes alone; its voices alone; everything; Foley without Earshot.
 */
@SuppressWarnings("UnstableApiUsage")
public class SceneRecording implements FabricClientGameTest {
	private static final int PHASE_TICKS = 160;

	@Override
	public void runTest(ClientGameTestContext context) {
		String mode = System.getenv("FOLEY_SCENE");
		if (mode == null) return;
		FoleyConfig all = new FoleyConfig(100, true, true, true, true, true, true, true, true, true, true, true, true,
				~0L);
		try (TestSingleplayerContext singleplayer = context.worldBuilder().create()) {
			singleplayer.getConnection().waitForChunksRender();
			var server = singleplayer.getServer();
			// Husks would kill the listener, and a dead one hears nothing.
			server.runCommand("gamemode creative @a");
			server.runCommand("time set noon");
			server.runCommand("weather clear");
			server.runCommand("gamerule doDaylightCycle false");
			for (String tree : new String[] {"~6 ~ ~6", "~-7 ~ ~5", "~9 ~ ~-4", "~-5 ~ ~-8", "~14 ~ ~10"}) {
				server.runCommand("place feature minecraft:oak " + tree);
			}
			server.runCommand("fill ~-12 ~-1 ~10 ~12 ~-1 ~10 water");
			server.runCommand("setblock ~-12 ~ ~10 water");
			server.runCommand("setblock ~4 ~ ~-3 campfire");
			String[][] crowd = {{"cow", "8"}, {"sheep", "8"}, {"chicken", "6"}, {"pig", "4"}, {"villager", "4"},
					{"husk", "4"}};
			for (String[] kind : crowd) {
				for (int i = 0; i < Integer.parseInt(kind[1]); i++) {
					int x = (i * 7 + kind[0].length() * 3) % 20 - 10;
					int z = (i * 5 + kind[0].length() * 7) % 20 - 10;
					server.runCommand("summon minecraft:" + kind[0] + " ~" + x + " ~ ~" + z);
				}
			}
			context.runOnClient(mc -> earshotHigh());
			context.waitTicks(100);
			if (mode.equals("earshot")) {
				// Earshot's parts one by one, over the game's own sounds.
				phase(context, "vanilla", () -> {
					FoleyClient.setBypassed(true);
					earshot(true);
				});
				phase(context, "earshot-dry-linear", () -> {
					earshotSet("withReverb", "OFF", false);
					earshot(false);
				});
				phase(context, "earshot-dry-realistic", () -> earshotSet("withReverb", "OFF", true));
				phase(context, "earshot-eax", () -> earshotSet("withReverb", "EAX", true));
				phase(context, "earshot-convolution", () -> earshotSet("withReverb", "CONVOLUTION", true));
				phase(context, "earshot-convolution-linear", () -> earshotSet("withReverb", "CONVOLUTION", false));
				FoleyClient.setBypassed(false);
				return;
			}
			FoleyConfig none = new FoleyConfig(100, false, false, false, false, false, false, false, false, false,
					false, false, false, 0);
			phase(context, "vanilla", () -> {
				FoleyClient.setBypassed(true);
				earshot(true);
			});
			phase(context, "earshot", () -> earshot(false));
			phase(context, "earshot+foley-ambience", () -> {
				FoleyClient.apply(new FoleyConfig(100, true, true, true, true, true, true, true, false, false, true,
						true, true, 0));
				FoleyClient.setBypassed(false);
			});
			phase(context, "earshot+foley-steps-strikes", () -> FoleyClient.apply(new FoleyConfig(100, false, false,
					false, false, false, false, false, true, true, false, false, false, 0)));
			phase(context, "earshot+foley-voices", () -> FoleyClient.apply(new FoleyConfig(100, false, false, false,
					false, false, false, false, false, false, false, false, false, ~0L)));
			phase(context, "everything", () -> FoleyClient.apply(all));
			phase(context, "foley-only", () -> earshot(true));
			FoleyClient.apply(none);
			FoleyClient.apply(all);
			earshot(false);
		}
	}

	/** A second of silence, then {@code change}, then the phase. */
	private static void phase(ClientGameTestContext context, String name, Runnable change) {
		context.runOnClient(mc -> mc.options.getSoundSourceOptionInstance(SoundSource.MASTER).set(0.0));
		context.runOnClient(mc -> change.run());
		context.waitTicks(20);
		context.runOnClient(mc -> mc.options.getSoundSourceOptionInstance(SoundSource.MASTER).set(1.0));
		FoleyClient.LOGGER.info("SCENE phase {} at {} ms", name, System.currentTimeMillis());
		context.waitTicks(PHASE_TICKS);
		FoleyClient.LOGGER.info("SCENE end {} at {} ms", name, System.currentTimeMillis());
	}

	/** Earshot's High preset with convolution reverb, if it's there. */
	private static void earshotHigh() {
		try {
			Class<?> client = Class.forName("io.github.cumsauce.earshot.EarshotClient");
			Object config = client.getMethod("config").invoke(null);
			Class<?> preset = Class.forName("io.github.cumsauce.earshot.config.EarshotConfig$Preset");
			Class<?> reverb = Class.forName("io.github.cumsauce.earshot.config.EarshotConfig$Reverb");
			Object next = config.getClass().getMethod("withPreset", preset).invoke(config,
					preset.getField("HIGH").get(null));
			next = next.getClass().getMethod("withReverb", reverb).invoke(next, reverb.getField("CONVOLUTION").get(null));
			client.getMethod("apply", next.getClass()).invoke(null, next);
		} catch (ReflectiveOperationException e) {
			FoleyClient.LOGGER.warn("SCENE no Earshot preset", e);
		}
	}

	/** Sets Earshot's reverb to {@code reverb} and its realistic falloff, if it's there. */
	private static void earshotSet(String method, String reverb, boolean realistic) {
		try {
			Class<?> client = Class.forName("io.github.cumsauce.earshot.EarshotClient");
			Object config = client.getMethod("config").invoke(null);
			Class<?> kind = Class.forName("io.github.cumsauce.earshot.config.EarshotConfig$Reverb");
			Object next = config.getClass().getMethod(method, kind).invoke(config, kind.getField(reverb).get(null));
			next = next.getClass().getMethod("withRealisticFalloff", boolean.class).invoke(next, realistic);
			client.getMethod("apply", next.getClass()).invoke(null, next);
		} catch (ReflectiveOperationException e) {
			FoleyClient.LOGGER.warn("SCENE no Earshot", e);
		}
	}

	/** Bypasses Earshot, if it's there (it isn't a dependency). */
	private static void earshot(boolean bypass) {
		try {
			Class.forName("io.github.cumsauce.earshot.EarshotClient").getMethod("setBypassed", boolean.class)
					.invoke(null, bypass);
		} catch (ReflectiveOperationException e) {
			// Not installed.
		}
	}
}
