package io.github.cumsauce.foley.ui;

import com.terraformersmc.modmenu.api.ConfigScreenFactory;
import com.terraformersmc.modmenu.api.ModMenuApi;

/**
 * Mod Menu's config button for Foley, opening the same settings as Music &amp; Sounds → Foley…. Fabric only loads this
 * class through the {@code modmenu} entrypoint, so Foley runs fine without Mod Menu installed.
 */
public final class ModMenuEntry implements ModMenuApi {
	@Override
	public ConfigScreenFactory<?> getModConfigScreenFactory() {
		return FoleyScreen::open;
	}
}
