package io.github.cumsauce.foley.gametest;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.api.Foley;
import io.github.cumsauce.foley.api.FoleyBeds;
import io.github.cumsauce.foley.api.FoleySounds;
import io.github.cumsauce.foley.api.SoundModel;
import io.github.cumsauce.foley.audio.AddonBeds;
import io.github.cumsauce.foley.audio.AddonSounds;
import io.github.cumsauce.foley.audio.CustomMaterials;
import io.github.cumsauce.foley.audio.Surfaces;
import io.github.cumsauce.foley.natives.NativeBridge;
import net.fabricmc.fabric.api.client.gametest.v1.FabricClientGameTest;
import net.fabricmc.fabric.api.client.gametest.v1.context.ClientGameTestContext;
import net.fabricmc.fabric.api.client.gametest.v1.context.TestSingleplayerContext;
import io.github.cumsauce.foley.audio.BedSound;
import io.github.cumsauce.foley.ui.FoleyScreen;
import io.github.cumsauce.foley.ui.SoundOptionsButton;
import net.minecraft.client.gui.screens.options.SoundOptionsScreen;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundEvents;
import net.minecraft.sounds.SoundSource;

/**
 * Starts the real client and checks: the native synthesis loaded; when it rains, Foley's rain (a synthesized stream)
 * plays through the game's sound engine, follows what it lands on and any roof, and replaces the game's rain sounds
 * (M1); the wind blows out in the open and drops under a roof (M1); a waterfall, a fire and a pool of lava each start
 * a bed placed where they are, and a dripstone drip is swapped for a generated one (M2); a lightning bolt strikes
 * Foley's thunder and the game's is skipped (M3); breaking glass and stepping down on stone layer Foley's modal
 * strikes and shatters under the game's sounds (M4), and a footstep too (M5); crickets chirp in the grass at night,
 * the bypass gives the game its sounds back, and the settings open from Music &amp; Sounds (M5); a sheep switched to
 * Foley's voice is voiced by it (M6), a nearby zombie breathes and a creeper's fuse hisses (M7), horses, goats, witches
 * and bees are Foley's (M8, M9); and a generated
 * one-shot (a drip) plays from the game's static sounds (M0), as do sounds the gametest itself defines through the
 * integration API, in code and in a resource pack (F0). CI has no sound card, so it runs with OpenAL Soft's null
 * backend.
 */
@SuppressWarnings("UnstableApiUsage")
public class FoleyClientGameTest implements FabricClientGameTest {
	@Override
	public void runTest(ClientGameTestContext context) {
		// Recording the listening scene instead ({@link SceneRecording}).
		if (System.getenv("FOLEY_SCENE") != null) return;
		if (FoleyClient.bridge() == null) throw new AssertionError("the native synthesis didn't load");
		try (TestSingleplayerContext singleplayer = context.worldBuilder().create()) {
			singleplayer.getConnection().waitForChunksRender();

			// The integration API (F0): sounds this gametest mod defines — from its foley entrypoint in code, and
			// from its resource pack — render as generated buffers and play, and registration has closed behind them.
			FoleySounds sounds = Foley.api().sounds();
			int beforeAddons = AddonSounds.rendered();
			try {
				context.waitFor(mc -> mc.player != null
						&& sounds.play(Identifier.fromNamespaceAndPath("foley-gametest", "code_drip"), mc.player.getX(),
								mc.player.getEyeY(), mc.player.getZ(), SoundSource.AMBIENT, 1f, 1f)
						&& sounds.play(Identifier.fromNamespaceAndPath("foley-gametest", "code_step"), mc.player.getX(),
								mc.player.getEyeY(), mc.player.getZ(), SoundSource.NEUTRAL, 1f, 1f)
						&& sounds.play(Identifier.fromNamespaceAndPath("foley-gametest", "code_chime"), mc.player.getX(),
								mc.player.getEyeY(), mc.player.getZ(), SoundSource.AMBIENT, 1f, 1f)
						&& sounds.play(Identifier.fromNamespaceAndPath("foley-gametest", "pack_drip"), mc.player.getX(),
								mc.player.getEyeY(), mc.player.getZ(), SoundSource.AMBIENT, 1f, 1f)
						&& sounds.play(Identifier.fromNamespaceAndPath("foley-gametest", "pack_howl"), mc.player.getX(),
								mc.player.getEyeY(), mc.player.getZ(), SoundSource.AMBIENT, 1f, 1f)
						&& AddonSounds.rendered() >= beforeAddons + 5, 20 * 20);
			} catch (RuntimeException | AssertionError e) {
				throw new AssertionError("an addon sound never rendered (rendered " + AddonSounds.rendered() + ")", e);
			}
			try {
				sounds.register(Identifier.fromNamespaceAndPath("foley-gametest", "late"), new SoundModel.LavaDrip());
				throw new AssertionError("registering after the entrypoint should have thrown");
			} catch (IllegalStateException expected) {
				// Registration closed when the entrypoints returned.
			}

			// The gametest's own material (F1): the amethyst's sound type and block mapped onto its crystal, so the
			// game's amethyst sounds layer Foley's physics for it.
			int beforeCustom = CustomMaterials.rendered();
			int beforeLayered = FoleyClient.layered();
			singleplayer.getServer().runCommand("setblock ~2 ~ ~ amethyst_block");
			try {
				context.waitFor(mc -> {
					if (mc.player != null) {
						double x = mc.player.getX() + 2, y = mc.player.getY(), z = mc.player.getZ();
						mc.getSoundManager().play(new SimpleSoundInstance(SoundEvents.AMETHYST_BLOCK_BREAK,
								SoundSource.BLOCKS, 1f, 1f, SoundInstance.createUnseededRandom(), x, y, z));
					}
					return FoleyClient.layered() > beforeLayered;
				}, 10 * 20);
			} catch (RuntimeException | AssertionError e) {
				throw new AssertionError("the game's amethyst break never got Foley's layer (layered "
						+ FoleyClient.layered() + ")", e);
			}
			try {
				context.waitFor(mc -> CustomMaterials.rendered() > beforeCustom, 10 * 20);
			} catch (RuntimeException | AssertionError e) {
				// The layer fired; where did it stop? Rendering the layer's buffer by hand says whether it's the
				// bridge or the delivery.
				var byHand = context.computeOnClient(mc -> io.github.cumsauce.foley.audio.GeneratedSounds.render(
						Identifier.fromNamespaceAndPath("foley", "sounds/generated/custom_0_3_0.ogg")));
				throw new AssertionError("the custom layer never rendered (layered " + FoleyClient.layered()
						+ ", rendered " + CustomMaterials.rendered() + ", by hand "
						+ (byHand == null ? "nothing" : byHand.size() / 2 + " samples") + ")", e);
			}

			// The gametest's own beds (F2): a water bed, told its scene, playing; a second starts, a third is
			// refused (at most two run), and the first retires.
			FoleyBeds beds = Foley.api().beds();
			Identifier brook = Identifier.fromNamespaceAndPath("foley-gametest", "brook");
			Identifier falls = Identifier.fromNamespaceAndPath("foley-gametest", "falls");
			boolean[] started = {false, false, false};
			context.runOnClient(mc -> {
				started[0] = beds.start(brook, FoleyBeds.BedKind.WATER, new float[] {20f, 0f, 1f});
				started[1] = beds.start(falls, FoleyBeds.BedKind.WATER, new float[] {0f, 40f, 1f});
				started[2] = beds.start(Identifier.fromNamespaceAndPath("foley-gametest", "third"),
						FoleyBeds.BedKind.WIND, new float[] {10f, 1f, 0.5f});
				beds.place(brook, mc.player.getX() + 3, mc.player.getY(), mc.player.getZ());
			});
			if (!started[0] || !started[1]) throw new AssertionError("the addon beds didn't start");
			if (started[2]) throw new AssertionError("a third bed started past the budget");
			try {
				context.waitFor(mc -> AddonBeds.rendered(brook) > NativeBridge.SAMPLE_RATE / 2, 10 * 20);
			} catch (RuntimeException | AssertionError e) {
				throw new AssertionError("the addon bed never rendered ("
						+ AddonBeds.rendered(brook) + " samples)", e);
			}
			context.runOnClient(mc -> {
				beds.scene(brook, new float[] {60f, 0f, 1f});
				beds.stop(brook);
				beds.stop(falls);
			});
			if (beds.playing(brook)) throw new AssertionError("the addon bed didn't retire");

			// The wind: out on a superflat, a breeze in the open.
			try {
				context.waitFor(mc -> {
					var wind = FoleyClient.wind();
					return wind != null && wind.sound() != null && mc.getSoundManager().isActive(wind.sound())
							&& wind.sound().rendered() > NativeBridge.SAMPLE_RATE && wind.scene().speed() > 1
							&& wind.scene().exposure() > 0.9f;
				}, 20 * 20);
			} catch (RuntimeException | AssertionError e) {
				var wind = FoleyClient.wind();
				throw new AssertionError("Foley's wind isn't blowing: " + (wind == null ? "off" : wind.scene() + ", sound "
						+ wind.sound()), e);
			}

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
			// The wind is sheltered from too.
			try {
				context.waitFor(mc -> FoleyClient.wind().scene().exposure() < 0.75f, 10 * 20);
			} catch (RuntimeException | AssertionError e) {
				throw new AssertionError("the roof doesn't shelter from the wind: " + FoleyClient.wind().scene(), e);
			}
			singleplayer.getServer().runCommand("weather clear");
			singleplayer.getServer().runCommand("fill ~-3 ~3 ~-3 ~3 ~3 ~3 air");

			// Water falling three blocks and spreading, fire on netherrack, and lava in a hole: each heard, placed.
			singleplayer.getServer().runCommand("setblock ~4 ~3 ~ water");
			singleplayer.getServer().runCommand("setblock ~-4 ~-1 ~ netherrack");
			singleplayer.getServer().runCommand("setblock ~-4 ~ ~ fire");
			singleplayer.getServer().runCommand("setblock ~ ~-1 ~6 lava");
			try {
				context.waitFor(mc -> {
					var placed = FoleyClient.placed();
					if (placed == null) return false;
					var around = placed.around();
					return around.water().falls() > 0 && around.water().flow() > 0 && around.fire().size() > 0
							&& around.lava().area() > 0 && playing(mc, placed.water()) && playing(mc, placed.fire())
							&& playing(mc, placed.lava());
				}, 20 * 20);
			} catch (RuntimeException | AssertionError e) {
				var placed = FoleyClient.placed();
				throw new AssertionError("water, fire and lava aren't all heard: "
						+ (placed == null ? "off" : placed.around()), e);
			}

			// A dripstone drip is swapped for one of Foley's.
			int before = FoleyClient.replaced();
			context.runOnClient(mc -> mc.getSoundManager().play(new SimpleSoundInstance(
					SoundEvents.POINTED_DRIPSTONE_DRIP_WATER, SoundSource.BLOCKS, 1f, 1f,
					SoundInstance.createUnseededRandom(), mc.player.getX(), mc.player.getY(), mc.player.getZ())));
			if (FoleyClient.replaced() <= before) throw new AssertionError("the game's drip wasn't replaced");

			// Thunder: a bolt 40 blocks off strikes Foley's thunder, which rumbles, and the game's is replaced.
			int replacedBefore = FoleyClient.replaced();
			singleplayer.getServer().runCommand("summon lightning_bolt ~40 ~ ~");
			try {
				context.waitFor(mc -> {
					var thunder = FoleyClient.thunder();
					return thunder != null && thunder.strikes() > 0 && playing(mc, thunder.sound())
							&& thunder.sound().rendered() > NativeBridge.SAMPLE_RATE
							&& FoleyClient.replaced() > replacedBefore;
				}, 20 * 20);
			} catch (RuntimeException | AssertionError e) {
				var thunder = FoleyClient.thunder();
				throw new AssertionError("no thunder: " + (thunder == null ? "off" : thunder.strikes() + " strikes, sound "
						+ thunder.sound() + ", replaced " + (FoleyClient.replaced() - replacedBefore)), e);
			}

			// Strikes and breaks: the game's glass breaking and a landing on stone get Foley's layer, rendered and playing.
			int layeredBefore = FoleyClient.layered();
			context.runOnClient(mc -> {
				var random = SoundInstance.createUnseededRandom();
				mc.getSoundManager().play(new SimpleSoundInstance(SoundEvents.GLASS_BREAK, SoundSource.BLOCKS, 1f, 1f,
						random, mc.player.getX(), mc.player.getY(), mc.player.getZ()));
				mc.getSoundManager().play(new SimpleSoundInstance(SoundEvents.STONE_FALL, SoundSource.BLOCKS, 1f, 1f,
						random, mc.player.getX(), mc.player.getY() + 1, mc.player.getZ()));
				mc.getSoundManager().play(new SimpleSoundInstance(SoundEvents.SNOW_STEP, SoundSource.PLAYERS, 1f, 1f,
						random, mc.player.getX(), mc.player.getY(), mc.player.getZ()));
			});
			if (FoleyClient.layered() < layeredBefore + 3) {
				throw new AssertionError("strikes weren't layered: " + (FoleyClient.layered() - layeredBefore));
			}

			// Life: at midnight on a superflat's grass, crickets chirp.
			singleplayer.getServer().runCommand("time set midnight");
			try {
				context.waitFor(mc -> FoleyClient.life().calls() > 0, 15 * 20);
			} catch (RuntimeException | AssertionError e) {
				throw new AssertionError("no crickets at night", e);
			}

			// Bypassed, Foley leaves the game's thunder alone.
			int beforeBypass = FoleyClient.replaced();
			context.runOnClient(mc -> {
				FoleyClient.setBypassed(true);
				mc.getSoundManager().play(new SimpleSoundInstance(SoundEvents.LIGHTNING_BOLT_THUNDER, SoundSource.WEATHER,
						1f, 1f, SoundInstance.createUnseededRandom(), mc.player.getX(), mc.player.getY(),
						mc.player.getZ()));
				FoleyClient.setBypassed(false);
			});
			if (FoleyClient.replaced() != beforeBypass) throw new AssertionError("bypassed, Foley still replaced thunder");

			// Mob voices: off by default; with the sheep switched to Foley's, a sheep's bleat is Foley's.
			var settings = FoleyClient.config();
			int beforeVoice = FoleyClient.replaced();
			context.runOnClient(mc -> mc.getSoundManager().play(new SimpleSoundInstance(SoundEvents.SHEEP_AMBIENT,
					SoundSource.NEUTRAL, 1f, 1f, SoundInstance.createUnseededRandom(), mc.player.getX(), mc.player.getY(),
					mc.player.getZ())));
			if (FoleyClient.replaced() != beforeVoice) throw new AssertionError("a voice was replaced by default");
			context.runOnClient(mc -> {
				FoleyClient.apply(new io.github.cumsauce.foley.config.FoleyConfig(settings.volume(), settings.rain(),
						settings.wind(), settings.thunder(), settings.water(), settings.fire(), settings.lava(),
						settings.drips(), settings.strikes(), settings.footsteps(), settings.birds(),
						settings.crickets(), settings.frogs(), settings.minecarts(), settings.doors(), settings.chests(),
						settings.redstone(), settings.boats(), settings.sculk(), 1 << 2));
				mc.getSoundManager().play(new SimpleSoundInstance(SoundEvents.SHEEP_AMBIENT, SoundSource.NEUTRAL, 1f,
						1.5f, SoundInstance.createUnseededRandom(), mc.player.getX(), mc.player.getY(),
						mc.player.getZ()));
				FoleyClient.apply(settings);
			});
			if (FoleyClient.replaced() != beforeVoice + 1) throw new AssertionError("the sheep wasn't voiced by Foley");

			// The gametest's own creature (F3): the dolphin, voiced by its descriptor, once "Addons' Creatures" is
			// switched on; off by default like every voice group.
			int beforeDolphin = FoleyClient.replaced();
			context.runOnClient(mc -> mc.getSoundManager().play(new SimpleSoundInstance(SoundEvents.DOLPHIN_AMBIENT,
					SoundSource.NEUTRAL, 1f, 1f, SoundInstance.createUnseededRandom(), mc.player.getX(),
					mc.player.getY(), mc.player.getZ())));
			if (FoleyClient.replaced() != beforeDolphin)
				throw new AssertionError("an addon creature was voiced by default");
			context.runOnClient(mc -> {
				FoleyClient.apply(new io.github.cumsauce.foley.config.FoleyConfig(settings.volume(), settings.rain(),
						settings.wind(), settings.thunder(), settings.water(), settings.fire(), settings.lava(),
						settings.drips(), settings.strikes(), settings.footsteps(), settings.birds(),
						settings.crickets(), settings.frogs(), settings.minecarts(), settings.doors(), settings.chests(),
						settings.redstone(), settings.boats(), settings.sculk(),
						1L << io.github.cumsauce.foley.audio.Voices.ADDON_GROUP));
				mc.getSoundManager().play(new SimpleSoundInstance(SoundEvents.DOLPHIN_AMBIENT, SoundSource.NEUTRAL,
						1f, 1f, SoundInstance.createUnseededRandom(), mc.player.getX(), mc.player.getY(),
						mc.player.getZ()));
				FoleyClient.apply(settings);
			});
			if (FoleyClient.replaced() != beforeDolphin + 1)
				throw new AssertionError("the dolphin wasn't voiced by Foley");

			// Voice engine 2 (M7): with zombies and creepers voiced, a zombie three blocks off breathes, and a creeper's
			// fuse is Foley's.
			context.runOnClient(mc -> FoleyClient.apply(new io.github.cumsauce.foley.config.FoleyConfig(
					settings.volume(), settings.rain(), settings.wind(), settings.thunder(), settings.water(),
					settings.fire(), settings.lava(), settings.drips(), settings.strikes(), settings.footsteps(),
					settings.birds(), settings.crickets(), settings.frogs(), settings.minecarts(), settings.doors(),
					settings.chests(), settings.redstone(), settings.boats(), settings.sculk(), (1 << 5) | (1 << 10))));
			singleplayer.getServer().runCommand("summon zombie ~3 ~ ~ {NoAI:1b,PersistenceRequired:1b}");
			try {
				context.waitFor(mc -> FoleyClient.breathing().breaths() > 0, 10 * 20);
			} catch (RuntimeException | AssertionError e) {
				throw new AssertionError("the zombie never breathed", e);
			}
			int beforeFuse = FoleyClient.replaced();
			context.runOnClient(mc -> mc.getSoundManager().play(new SimpleSoundInstance(SoundEvents.CREEPER_PRIMED,
					SoundSource.HOSTILE, 1f, 1f, SoundInstance.createUnseededRandom(), mc.player.getX(), mc.player.getY(),
					mc.player.getZ())));
			if (FoleyClient.replaced() != beforeFuse + 1) throw new AssertionError("the creeper's fuse wasn't Foley's");
			singleplayer.getServer().runCommand("kill @e[type=zombie]");

			// Everyday animals and rarer hostiles (M8, M9): a horse, a screaming goat and a witch's cackle are Foley's,
			// and a bee's buzz is pointed at Foley's loop.
			long more = 0;
			for (String group : java.util.List.of("horse", "goat", "witch", "bee")) {
				more |= 1L << io.github.cumsauce.foley.audio.Voices.GROUPS.indexOf(group);
			}
			long chosen = more;
			context.runOnClient(mc -> FoleyClient.apply(new io.github.cumsauce.foley.config.FoleyConfig(
					settings.volume(), settings.rain(), settings.wind(), settings.thunder(), settings.water(),
					settings.fire(), settings.lava(), settings.drips(), settings.strikes(), settings.footsteps(),
					settings.birds(), settings.crickets(), settings.frogs(), settings.minecarts(), settings.doors(),
					settings.chests(), settings.redstone(), settings.boats(), settings.sculk(), chosen)));
			int beforeAnimals = FoleyClient.replaced();
			int beforeRedirect = io.github.cumsauce.foley.audio.Voices.redirected();
			context.runOnClient(mc -> {
				for (var event : java.util.List.of(SoundEvents.HORSE_AMBIENT, SoundEvents.GOAT_SCREAMING_AMBIENT,
						SoundEvents.WITCH_CELEBRATE, SoundEvents.BEE_LOOP)) {
					mc.getSoundManager().play(new SimpleSoundInstance(event, SoundSource.NEUTRAL, 1f, 1f,
							SoundInstance.createUnseededRandom(), mc.player.getX(), mc.player.getY(), mc.player.getZ()));
				}
			});
			if (FoleyClient.replaced() != beforeAnimals + 3) {
				throw new AssertionError("horse, goat and witch: " + (FoleyClient.replaced() - beforeAnimals) + " voiced");
			}
			if (io.github.cumsauce.foley.audio.Voices.redirected() <= beforeRedirect) {
				throw new AssertionError("the bee's buzz wasn't Foley's");
			}
			context.runOnClient(mc -> FoleyClient.apply(settings));

			// Active structures (M21): a door, a chest, and a sculk sensor each fire Foley's event.
			int beforeStructures = FoleyClient.replaced();
			singleplayer.getServer().runCommand("setblock ~3 ~ ~ oak_door[facing=north,hinge=left,half=lower]");
			context.runOnClient(mc -> mc.getSoundManager().play(new SimpleSoundInstance(SoundEvents.WOODEN_DOOR_OPEN,
					SoundSource.BLOCKS, 1f, 1f, SoundInstance.createUnseededRandom(), mc.player.getX() + 3,
					mc.player.getY(), mc.player.getZ())));
			singleplayer.getServer().runCommand("setblock ~4 ~ ~ chest");
			context.runOnClient(mc -> mc.getSoundManager().play(new SimpleSoundInstance(SoundEvents.CHEST_OPEN,
					SoundSource.BLOCKS, 1f, 1f, SoundInstance.createUnseededRandom(), mc.player.getX() + 4,
					mc.player.getY(), mc.player.getZ())));
			singleplayer.getServer().waitFor(server -> server.getTickCount() > 0, 60);
			if (FoleyClient.replaced() < beforeStructures + 2) {
				throw new AssertionError("M21 door/chest did not fire (replaced delta="
						+ (FoleyClient.replaced() - beforeStructures) + ")");
			}
			// All toggles off: M21 stays silent.
			int beforeMuted = FoleyClient.replaced();
			context.runOnClient(mc -> FoleyClient.apply(new io.github.cumsauce.foley.config.FoleyConfig(
					settings.volume(), false, false, false, false, false, false, false, false, false,
					false, false, false, false, false, false, false, false, false, 0L)));
			context.runOnClient(mc -> mc.getSoundManager().play(new SimpleSoundInstance(SoundEvents.CHEST_OPEN,
					SoundSource.BLOCKS, 1f, 1f, SoundInstance.createUnseededRandom(), mc.player.getX() + 4,
					mc.player.getY(), mc.player.getZ())));
			singleplayer.getServer().waitFor(server -> server.getTickCount() > 0, 60);
			if (FoleyClient.replaced() != beforeMuted) {
				throw new AssertionError("M21 played when its toggles were off (replaced delta="
						+ (FoleyClient.replaced() - beforeMuted) + ")");
			}
			context.runOnClient(mc -> FoleyClient.apply(settings));

			// The settings, where players look first: Options → Music & Sounds → Foley.
			context.setScreen(() -> new SoundOptionsScreen(null, Minecraft.getInstance().options));
			// The button is inside the screen's options list, where clickScreenButton doesn't look.
			context.runOnClient(mc -> SoundOptionsButton.find(mc.gui.screen())
					.orElseThrow(() -> new AssertionError("no Foley button in Music & Sounds"))
					.onPress(new net.minecraft.client.input.KeyEvent(
							com.mojang.blaze3d.platform.InputConstants.KEY_RETURN, 0, 0)));
			context.waitForScreen(FoleyScreen.class);
			context.takeScreenshot("foley-settings");
			context.setScreen(() -> null);

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

	private static boolean playing(Minecraft minecraft, BedSound<?> sound) {
		return sound != null && minecraft.getSoundManager().isActive(sound) && sound.rendered() > 0;
	}
}
