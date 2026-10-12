package io.github.cumsauce.foley.gametest;

import io.github.cumsauce.foley.FoleyClient;
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
import net.minecraft.sounds.SoundEvents;
import net.minecraft.sounds.SoundSource;

/**
 * Starts the real client and checks: the native synthesis loaded; when it rains, Foley's rain (a synthesized stream)
 * plays through the game's sound engine, follows what it lands on and any roof, and replaces the game's rain sounds
 * (M1); the wind blows out in the open and drops under a roof (M1); a waterfall, a fire and a pool of lava each start
 * a bed placed where they are, and a dripstone drip is swapped for a generated one (M2); a lightning bolt strikes
 * Foley's thunder and the game's is skipped (M3); breaking glass and stepping down on stone layer Foley's modal
 * strikes and shatters under the game's sounds (M4), and a footstep too (M5); crickets chirp in the grass at night,
 * the bypass gives the game its sounds back, and the settings open from Music &amp; Sounds (M5); and a generated
 * one-shot (a drip) plays from the game's static sounds (M0). CI has no sound card, so it runs with OpenAL Soft's null
 * backend.
 */
@SuppressWarnings("UnstableApiUsage")
public class FoleyClientGameTest implements FabricClientGameTest {
	@Override
	public void runTest(ClientGameTestContext context) {
		if (FoleyClient.bridge() == null) throw new AssertionError("the native synthesis didn't load");
		try (TestSingleplayerContext singleplayer = context.worldBuilder().create()) {
			singleplayer.getConnection().waitForChunksRender();

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
