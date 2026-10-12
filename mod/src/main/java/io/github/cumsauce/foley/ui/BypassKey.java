package io.github.cumsauce.foley.ui;

import com.mojang.blaze3d.platform.InputConstants;
import io.github.cumsauce.foley.FoleyClient;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.keymapping.v1.KeyMappingHelper;
import net.minecraft.client.KeyMapping;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;

/**
 * A key, unbound until the player binds it, that switches Foley off and on again at once, for listening to the
 * difference: off, every sound it replaces is the game's again and everything it adds stops. The action bar says
 * which it is.
 */
public final class BypassKey {
	private static KeyMapping key;

	private BypassKey() {}

	/** The key, once registered. */
	public static KeyMapping key() {
		return key;
	}

	public static void register() {
		KeyMapping.Category category = KeyMapping.Category.register(Identifier.fromNamespaceAndPath("foley", "keys"));
		key = KeyMappingHelper.registerKeyMapping(
				new KeyMapping("key.foley.bypass", InputConstants.UNKNOWN.getValue(), category));
		ClientTickEvents.END_CLIENT_TICK.register(minecraft -> {
			boolean pressed = false;
			while (key.consumeClick()) pressed = !pressed;
			if (!pressed || FoleyClient.bridge() == null) return;
			FoleyClient.setBypassed(!FoleyClient.bypassed());
			if (minecraft.player != null) {
				minecraft.player.sendOverlayMessage(Component.translatable(
						FoleyClient.bypassed() ? "foley.bypass.on" : "foley.bypass.off"));
			}
		});
	}
}
