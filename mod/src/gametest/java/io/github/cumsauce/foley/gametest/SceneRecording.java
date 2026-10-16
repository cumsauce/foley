package io.github.cumsauce.foley.gametest;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import net.fabricmc.fabric.api.client.gametest.v1.FabricClientGameTest;
import net.fabricmc.fabric.api.client.gametest.v1.context.ClientGameTestContext;
import net.fabricmc.fabric.api.client.gametest.v1.context.TestSingleplayerContext;
import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.level.biome.Biomes;
import net.minecraft.world.phys.Vec3;

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
		if (mode.equals("walk")) {
			walk(context);
			return;
		}
		if (mode.equals("idle")) {
			idle(context);
			return;
		}
		FoleyConfig all = FoleyConfig.ALL;
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
			FoleyConfig none = FoleyConfig.NONE;
			phase(context, "vanilla", () -> {
				FoleyClient.setBypassed(true);
				earshot(true);
			});
			phase(context, "earshot", () -> earshot(false));
			phase(context, "earshot+foley-ambience", () -> {
				FoleyClient.apply(new FoleyConfig(100, true, true, true, true, true, true, true, false, false, true,
						true, true, true, true, true, true, true, true, true, true, true, true, 0)
						.withSettling(true).withMobBrushes(true).withPeacefulBreathing(true).withAshDrift(true));
				FoleyClient.setBypassed(false);
			});
			phase(context, "earshot+foley-steps-strikes", () -> FoleyClient.apply(new FoleyConfig(100, false, false,
					false, false, false, false, false, true, true, false, false, false, true, true, true, true, true,
					true, true, true, true, true, 0)
					.withSettling(true).withMobBrushes(true).withPeacefulBreathing(true).withAshDrift(true)));
			phase(context, "earshot+foley-voices", () -> FoleyClient.apply(new FoleyConfig(100, false, false, false,
					false, false, false, false, false, false, false, false, false, true, true, true, true, true,
					true, true, true, true, true, ~0L)
					.withSettling(true).withMobBrushes(true).withPeacefulBreathing(true).withAshDrift(true)));
			phase(context, "everything", () -> FoleyClient.apply(all));
			phase(context, "foley-only", () -> earshot(true));
			FoleyClient.apply(none);
			FoleyClient.apply(all);
			earshot(false);
		}
	}

	/**
	 * A walk in a generated world, as a player goes: sprinting and jumping up a wooded hillside past a waterfall,
	 * turning, then breaking and placing blocks; the same route on EAX and then on convolution, each in two phases
	 * ("walk-*" and "hit-*").
	 */
	private static void walk(ClientGameTestContext context) {
		try (TestSingleplayerContext singleplayer = context.worldBuilder().setUseConsistentSettings(false)
				.adjustSettings(settings -> settings.setSeed("foley")).create()) {
			var server = singleplayer.getServer();
			BlockPos forest = server.computeOnServer(game -> {
				ServerLevel level = game.overworld();
				var found = level.findClosestBiome3d(biome -> biome.is(Biomes.FOREST), BlockPos.ZERO.atY(64), 3200, 32,
						64);
				return found == null ? BlockPos.ZERO : found.getFirst();
			});
			server.runCommand("gamemode creative @a");
			server.runCommand("time set noon");
			server.runCommand("weather clear");
			server.runCommand("gamerule doDaylightCycle false");
			server.runCommand("gamerule doWeatherCycle false");
			server.runCommand("spreadplayers " + forest.getX() + " " + forest.getZ() + " 0 1 false @a");
			singleplayer.getConnection().waitForChunksRender();
			Vec3 start = context.computeOnClient(mc -> mc.player.position());
			FoleyClient.LOGGER.info("SCENE walking from {}", start);
			// Rushing water: falls off the hillside ahead.
			server.runCommand("execute as @p at @p run setblock ~3 ~6 ~8 water");
			server.runCommand("execute as @p at @p run setblock ~-4 ~5 ~14 water");
			server.runCommand("item replace entity @p weapon.mainhand with stone 64");
			context.runOnClient(mc -> earshotHigh());
			context.waitTicks(100);
			FoleyConfig every = FoleyConfig.ALL;
			// (name, reverb, Foley's settings or null for none): the last three take the mix apart.
			Object[][] runs = {{"eax", "EAX", every}, {"convolution", "CONVOLUTION", every},
					{"nowind", "EAX", every.withWind(false)}, {"nowater", "EAX", every.withWater(false)},
					{"nofoley", "EAX", null}};
			for (Object[] run : runs) {
				String reverb = (String) run[1];
				FoleyConfig foley = (FoleyConfig) run[2];
				context.runOnClient(mc -> mc.options.getSoundSourceOptionInstance(SoundSource.MASTER).set(0.0));
				server.runCommand(String.format(java.util.Locale.ROOT, "tp @a %.2f %.2f %.2f 0 0", start.x, start.y,
						start.z));
				context.runOnClient(mc -> {
					earshotSet("withReverb", reverb, true);
					FoleyClient.setBypassed(foley == null);
					FoleyClient.apply(foley == null ? every : foley);
				});
				context.waitTicks(40);
				context.runOnClient(mc -> mc.options.getSoundSourceOptionInstance(SoundSource.MASTER).set(1.0));
				String name = (String) run[0];
				FoleyClient.LOGGER.info("SCENE phase walk-{} at {} ms", name, System.currentTimeMillis());
				var input = context.getInput();
				input.holdKey(options -> options.keyUp);
				input.holdKey(options -> options.keySprint);
				for (int tick = 0; tick < PHASE_TICKS; tick++) {
					if (tick % 12 == 0) input.pressKey(options -> options.keyJump);
					input.lookAt(40f * (float) Math.sin(tick / 25.0), 10f * (float) Math.sin(tick / 9.0));
					context.waitTick();
					if (tick % 40 == 20) {
						context.runOnClient(mc -> FoleyClient.LOGGER.info("SCENE at {} {} water {}",
								mc.player.position(), FoleyClient.wind() == null ? "" : FoleyClient.wind().scene(),
								FoleyClient.placed() == null || FoleyClient.placed().water() == null ? "none"
										: FoleyClient.placed().water().scene()));
					}
				}
				input.releaseKey(options -> options.keySprint);
				input.releaseKey(options -> options.keyUp);
				FoleyClient.LOGGER.info("SCENE end walk-{} at {} ms", name, System.currentTimeMillis());
				if (!name.equals("eax") && !name.equals("convolution")) continue;
				context.waitTicks(20);
				FoleyClient.LOGGER.info("SCENE phase hit-{} at {} ms", name, System.currentTimeMillis());
				input.lookAt(0f, 70f);
				for (int hit = 0; hit < 8; hit++) {
					input.pressMouse(hit % 2 == 0 ? 0 : 1);
					context.waitTicks(10);
				}
				FoleyClient.LOGGER.info("SCENE end hit-{} at {} ms", name, System.currentTimeMillis());
			}
		}
	}

	/** Wandering in a forest for minutes, with Earshot and then without: for faults that come with time. */
	private static void idle(ClientGameTestContext context) {
		try (TestSingleplayerContext singleplayer = context.worldBuilder().setUseConsistentSettings(false)
				.adjustSettings(settings -> settings.setSeed("foley")).create()) {
			var server = singleplayer.getServer();
			BlockPos forest = server.computeOnServer(game -> {
				var found = game.overworld().findClosestBiome3d(b -> b.is(Biomes.FOREST), BlockPos.ZERO.atY(64), 3200, 32, 64);
				return found == null ? BlockPos.ZERO : found.getFirst();
			});
			for (String c : new String[] {"gamemode creative @a", "time set noon", "weather clear",
					"gamerule doDaylightCycle false", "gamerule doWeatherCycle false"}) {
				server.runCommand(c);
			}
			server.runCommand("spreadplayers " + forest.getX() + " " + forest.getZ() + " 0 1 false @a");
			singleplayer.getConnection().waitForChunksRender();
			context.runOnClient(mc -> earshotHigh());
			context.waitTicks(100);
			for (boolean bypass : new boolean[] {false, true}) {
				context.runOnClient(mc -> earshot(bypass));
				String name = bypass ? "idle-bare" : "idle-earshot";
				FoleyClient.LOGGER.info("SCENE phase {} at {} ms", name, System.currentTimeMillis());
				long end = System.currentTimeMillis() + 240_000;
				// Wandering: walking, turning, now and then sprinting and jumping.
				var input = context.getInput();
				input.holdKey(options -> options.keyUp);
				for (int tick = 0; System.currentTimeMillis() < end; tick++) {
					input.lookAt(tick * 1.5f % 360, 0);
					if (tick % 40 == 0) input.pressKey(options -> options.keyJump);
					if (tick % 200 == 0) input.holdKey(options -> options.keySprint);
					if (tick % 200 == 100) input.releaseKey(options -> options.keySprint);
					context.waitTick();
				}
				input.releaseKey(options -> options.keyUp);
				input.releaseKey(options -> options.keySprint);
				FoleyClient.LOGGER.info("SCENE end {} at {} ms", name, System.currentTimeMillis());
			}
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
