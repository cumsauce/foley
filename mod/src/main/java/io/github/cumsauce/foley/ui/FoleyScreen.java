package io.github.cumsauce.foley.ui;

import com.mojang.serialization.Codec;
import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import java.util.List;
import net.minecraft.ChatFormatting;
import net.minecraft.client.Minecraft;
import net.minecraft.client.OptionInstance;
import net.minecraft.client.Options;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.client.gui.components.MultiLineTextWidget;
import net.minecraft.client.gui.screens.AlertScreen;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.options.OptionsSubScreen;
import net.minecraft.network.chat.Component;

/**
 * Foley's settings, under Options → Music &amp; Sounds, in sections: its volume; weather; water, fire and lava; strikes
 * and steps; life; and mob voices (experimental, each mob off until chosen). Each sound Foley replaces can be given back to the game ("Foley" or "The game's"), and each it
 * adds switched off. Changes apply and are saved when the screen closes.
 */
public final class FoleyScreen extends OptionsSubScreen {
	private static final Component TITLE = Component.translatable("foley.settings.title");
	private static final int TEXT_WIDTH = 310;

	private final FoleyConfig initial;
	private final OptionInstance<Integer> volume;
	private final OptionInstance<Boolean> rain;
	private final OptionInstance<Boolean> wind;
	private final OptionInstance<Boolean> thunder;
	private final OptionInstance<Boolean> water;
	private final OptionInstance<Boolean> fire;
	private final OptionInstance<Boolean> lava;
	private final OptionInstance<Boolean> drips;
	private final OptionInstance<Boolean> strikes;
	private final OptionInstance<Boolean> footsteps;
	private final OptionInstance<Boolean> birds;
	private final OptionInstance<Boolean> crickets;
	private final OptionInstance<Boolean> frogs;
	private final OptionInstance<Boolean> minecarts;
	private final OptionInstance<Boolean> doors;
	private final OptionInstance<Boolean> chests;
	private final OptionInstance<Boolean> redstone;
	private final OptionInstance<Boolean> boats;
	private final OptionInstance<Boolean> sculk;
	private final OptionInstance<Boolean> caveAtmosphere;
	private final OptionInstance<Boolean> rarerSpecies;
	private final OptionInstance<Boolean> underwater;
	private final OptionInstance<Boolean> aeolian;
	private final List<OptionInstance<Boolean>> voices = new java.util.ArrayList<>();
	/** The settings over {@code parent}, or an alert if Foley's synthesis isn't running. */
	public static Screen open(Screen parent) {
		Minecraft minecraft = Minecraft.getInstance();
		if (FoleyClient.bridge() == null) {
			return new AlertScreen(() -> minecraft.gui.setScreen(parent), TITLE,
					Component.translatable("foley.settings.unavailable"));
		}
		return new FoleyScreen(parent, minecraft.options, FoleyClient.config());
	}

	private FoleyScreen(Screen parent, Options options, FoleyConfig config) {
		super(parent, options, TITLE);
		initial = config;
		volume = new OptionInstance<>("foley.settings.volume",
				OptionInstance.cachedConstantTooltip(Component.translatable("foley.settings.volume.tooltip")),
				(caption, value) -> Component.translatable("options.percent_value", caption, value),
				new OptionInstance.IntRange(0, 100), config.volume(), OptionInstance.NO_ACTION);
		rain = replaced("foley.settings.rain", config.rain());
		wind = added("foley.settings.wind", config.wind());
		thunder = replaced("foley.settings.thunder", config.thunder());
		water = replaced("foley.settings.water", config.water());
		fire = replaced("foley.settings.fire", config.fire());
		lava = replaced("foley.settings.lava", config.lava());
		drips = replaced("foley.settings.drips", config.drips());
		strikes = added("foley.settings.strikes", config.strikes());
		footsteps = added("foley.settings.footsteps", config.footsteps());
		birds = added("foley.settings.birds", config.birds());
		crickets = added("foley.settings.crickets", config.crickets());
		frogs = added("foley.settings.frogs", config.frogs());
		minecarts = added("foley.settings.minecarts", config.minecarts());
		doors = added("foley.settings.doors", config.doors());
		chests = added("foley.settings.chests", config.chests());
		redstone = added("foley.settings.redstone", config.redstone());
		boats = added("foley.settings.boats", config.boats());
		sculk = added("foley.settings.sculk", config.sculk());
		caveAtmosphere = added("foley.settings.caveAtmosphere", config.caveAtmosphere());
		rarerSpecies = added("foley.settings.rarerSpecies", config.rarerSpecies());
		underwater = added("foley.settings.underwater", config.underwater());
		aeolian = added("foley.settings.aeolian", config.aeolian());
		for (int i = 0; i < io.github.cumsauce.foley.audio.Voices.GROUPS.size(); i++) {			String mob = io.github.cumsauce.foley.audio.Voices.GROUPS.get(i);
			voices.add(new OptionInstance<>("foley.settings.voice." + mob,
					OptionInstance.cachedConstantTooltip(Component.translatable("foley.settings.voice.tooltip")),
					(caption, value) -> Component.translatable(
							value ? "foley.settings.made_by.foley" : "foley.settings.made_by.game"),
					new OptionInstance.Enum<>(List.of(true, false), Codec.BOOL), config.voice(i),
					OptionInstance.NO_ACTION));
		}
	}

	/** A sound Foley replaces: "Foley" or "The game's". */
	private static OptionInstance<Boolean> replaced(String key, boolean initial) {
		return new OptionInstance<>(key,
				OptionInstance.cachedConstantTooltip(Component.translatable(key + ".tooltip")),
				// The cycle button puts the caption before it.
				(caption, value) -> Component.translatable(
						value ? "foley.settings.made_by.foley" : "foley.settings.made_by.game"),
				new OptionInstance.Enum<>(List.of(true, false), Codec.BOOL), initial, OptionInstance.NO_ACTION);
	}

	/** A sound Foley adds: on or off. */
	private static OptionInstance<Boolean> added(String key, boolean initial) {
		return OptionInstance.createBoolean(key,
				OptionInstance.cachedConstantTooltip(Component.translatable(key + ".tooltip")), initial);
	}

	@Override
	protected void addOptions() {
		list.addBig(volume);

		list.addHeader(Component.translatable("foley.settings.section.weather"));
		list.addSmall(rain, thunder);
		list.addSmall(wind, underwater);

		list.addHeader(Component.translatable("foley.settings.section.elements"));
		list.addSmall(water, drips);
		list.addSmall(fire, lava);
		list.addSmall(caveAtmosphere, null);

		list.addHeader(Component.translatable("foley.settings.section.contact"));
		list.addSmall(strikes, footsteps);

		list.addHeader(Component.translatable("foley.settings.section.life"));
		list.addSmall(birds, crickets);
		list.addSmall(frogs, rarerSpecies);

		list.addHeader(Component.translatable("foley.settings.section.structures"));
		list.addSmall(minecarts, doors);
		list.addSmall(chests, redstone);
		list.addSmall(boats, sculk);
		list.addSmall(aeolian, null);

		list.addHeader(Component.translatable("foley.settings.section.voices"));
		list.addBig(note(Component.translatable("foley.settings.voices.note")));
		boolean allFoley = voices.stream().allMatch(OptionInstance::get);
		list.addBig(net.minecraft.client.gui.components.Button.builder(
				Component.translatable(allFoley ? "foley.settings.voices.all_game" : "foley.settings.voices.all_foley"),
				button -> {
					voices.forEach(v -> v.set(!allFoley));
					rebuildWidgets();
				}).build());
		for (int i = 0; i < voices.size(); i += 2) {
			list.addSmall(voices.get(i), i + 1 < voices.size() ? voices.get(i + 1) : null);
		}

		list.addBig(note(bypassHint()));
		updateActive();
	}

	/** A grey line of text, centred in the list's full width. */
	private MultiLineTextWidget note(Component text) {
		MultiLineTextWidget note = new MultiLineTextWidget(text.copy().withStyle(ChatFormatting.GRAY), font) {
			// It sizes itself to its text, so it would sit at the row's left edge; as wide as the row, it centres.
			@Override
			public int getWidth() {
				return TEXT_WIDTH;
			}
		};
		return note.setMaxWidth(TEXT_WIDTH).setCentered(true);
	}

	@Override
	public void tick() {
		super.tick();
		updateActive();
	}

	/** At no volume, there's nothing to set. */
	private void updateActive() {
		boolean audible = volume.get() > 0;
		List<OptionInstance<?>> all = new java.util.ArrayList<>(List.of(rain, wind, thunder, water, fire, lava, drips,
				strikes, footsteps, birds, crickets, frogs, underwater));
		all.addAll(voices);
		for (OptionInstance<?> option : all) {
			AbstractWidget widget = list.findOption(option);
			if (widget != null) widget.active = audible;
		}
	}

	/** Where the A/B key is, or how to bind it. */
	private static Component bypassHint() {
		var key = BypassKey.key();
		return key == null || key.isUnbound() ? Component.translatable("foley.settings.bypass.unbound")
				: Component.translatable("foley.settings.bypass.bound", key.getTranslatedKeyMessage());
	}

	@Override
	public void removed() {
		long voiced = 0;
		for (int i = 0; i < voices.size(); i++) {
			if (voices.get(i).get()) voiced |= 1L << i;
		}
		FoleyConfig next = new FoleyConfig(volume.get(), rain.get(), wind.get(), thunder.get(), water.get(), fire.get(),
				lava.get(), drips.get(), strikes.get(), footsteps.get(), birds.get(), crickets.get(), frogs.get(),
				minecarts.get(), doors.get(), chests.get(), redstone.get(), boats.get(), sculk.get(),
				caveAtmosphere.get(), rarerSpecies.get(), underwater.get(), aeolian.get(), voiced);
		if (!next.equals(initial)) FoleyClient.apply(next);		super.removed();
	}
}
