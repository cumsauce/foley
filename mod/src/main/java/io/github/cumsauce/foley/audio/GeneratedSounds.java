package io.github.cumsauce.foley.audio;

import com.mojang.blaze3d.audio.SoundBuffer;
import io.github.cumsauce.foley.natives.NativeBridge;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import net.minecraft.resources.Identifier;
import org.jspecify.annotations.Nullable;

/**
 * One-shots rendered into ordinary sound buffers, under names in {@code assets/foley/sounds.json} that have no file
 * ({@code foley:generated/<model>_<n>}), so they play from the game's large pool of static sources like any sound. The
 * game asks its buffer library for them; {@code SoundBufferLibraryMixin} answers with {@link #render}. Each name is a
 * different variation (a seed), and the game picks among them at random.
 */
public final class GeneratedSounds {
	public static final String PREFIX = "generated/";
	private static @Nullable NativeBridge bridge;

	private GeneratedSounds() {}

	public static void use(NativeBridge bridge) {
		GeneratedSounds.bridge = bridge;
	}

	/** Whether {@code sound} (a sound's location, as sounds.json names it) is one Foley generates. */
	public static boolean isGenerated(Identifier sound) {
		return sound.getNamespace().equals("foley") && sound.getPath().startsWith(PREFIX);
	}

	/**
	 * The buffer for {@code path}, as the buffer library asks for it ({@code foley:sounds/generated/drip_3.ogg}), or
	 * null if it isn't one of Foley's.
	 */
	public static @Nullable SoundBuffer render(Identifier path) {
		NativeBridge bridge = GeneratedSounds.bridge;
		String name = path.getPath();
		if (bridge == null || !path.getNamespace().equals("foley") || !name.startsWith("sounds/" + PREFIX)) return null;
		name = name.substring(("sounds/" + PREFIX).length()).replace(".ogg", "");
		int split = name.lastIndexOf('_');
		String model = name.substring(0, split);
		int variant = Integer.parseInt(name.substring(split + 1));
		short[] pcm = switch (model) {
			// Drips trap bubbles of about 0.6 to 3 mm, spread over the variants.
			case "drip" -> bridge.renderDrip(0.6f * (float) Math.pow(5, variant / 15.0), 0x5EED + variant);
			default -> null;
		};
		if (pcm == null) return null;
		ByteBuffer data = ByteBuffer.allocateDirect(pcm.length * 2).order(ByteOrder.LITTLE_ENDIAN);
		data.asShortBuffer().put(pcm);
		return new SoundBuffer(data, BedSound.FORMAT);
	}
}
