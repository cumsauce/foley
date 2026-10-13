package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.natives.NativeBridge;
import net.minecraft.client.Minecraft;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.phys.Vec3;
import org.jspecify.annotations.Nullable;

/**
 * A bed heard all round the listener (wind, rain) as {@value #AROUND} beds, each its own rendering (so decorrelated:
 * Kendall 1995, Potard and Burnett 2004), placed round the ear in the four quarters of the world and weighted by how
 * open each quarter is. One mono bed at the ear sounds inside the head; uncorrelated ones round it sound like a field
 * the listener stands in, and louder from the open side (the rain past a roof's edge). The weights' squares sum to 1,
 * so it is as loud as one bed was.
 *
 * @param <S> what the beds are told about the world
 */
public final class SurroundBeds<S extends Scene> {
	/** How many beds, and quarters: -x -z, +x -z, -x +z, +x +z. */
	public static final int AROUND = 4;
	/** How far from the ear they play, blocks. */
	private static final double DISTANCE = 2;
	/** Even a closed-in quarter gets some: the field reaches round corners. */
	private static final float FLOOR = 0.15f;

	private final String name;
	private final SoundSource source;
	private final NativeBridge bridge;
	private final int kind;
	@SuppressWarnings("unchecked")
	private final @Nullable BedSound<S>[] beds = new BedSound[AROUND];

	public SurroundBeds(String name, SoundSource source, NativeBridge bridge, int kind) {
		this.name = name;
		this.source = source;
		this.bridge = bridge;
		this.kind = kind;
	}

	/** The quarter (0 to 3) an offset of {@code dx}, {@code dz} from the listener is in. */
	public static int quarter(double dx, double dz) {
		return (dx >= 0 ? 1 : 0) + (dz >= 0 ? 2 : 0);
	}

	/** Where quarter {@code q}'s bed plays from, relative to the ear, in world axes. */
	static Vec3 offset(int q) {
		double s = DISTANCE * Math.sqrt(0.5);
		return new Vec3((q & 1) != 0 ? s : -s, 0, (q & 2) != 0 ? s : -s);
	}

	/** Keeps all four playing {@code scene}, weighted by {@code open} (per quarter, 0 to 1). Client thread. */
	public void keep(Minecraft minecraft, S scene, float[] open) {
		float sum = 0;
		for (float o : open) sum += FLOOR + o;
		for (int q = 0; q < AROUND; q++) {
			float weight = (float) Math.sqrt((FLOOR + open[q]) / sum);
			BedSound<S> bed = beds[q];
			if (bed == null || !minecraft.getSoundManager().isActive(bed)) {
				bed = new BedSound<>(name, source, bridge, kind, scene).follow(offset(q), weight);
				beds[q] = bed;
				minecraft.getSoundManager().play(bed);
			} else {
				bed.setScene(scene);
				bed.weigh(weight);
			}
		}
	}

	/** Tells the beds already playing about {@code scene}, starting none. Client thread. */
	public void setScene(S scene) {
		for (BedSound<S> bed : beds) {
			if (bed != null) bed.setScene(scene);
		}
	}

	public void stop(Minecraft minecraft) {
		for (int q = 0; q < AROUND; q++) {
			if (beds[q] != null) minecraft.getSoundManager().stop(beds[q]);
			beds[q] = null;
		}
	}

	/** One of the beds, if they're playing. */
	public @Nullable BedSound<S> any() {
		return beds[0];
	}
}
