package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.natives.NativeBridge;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import org.jspecify.annotations.Nullable;

/**
 * One state-bearing slot per known village bell, so the controller can rate-limit per
 * bell (the bell rings at most every few seconds; the murmur fires every few seconds per
 * bell; the smith is per anvil). Slots are pruned when their bell leaves the
 * {@code RADIUS}. The cap {@link #MAX_BEDS} keeps a city of many villages inside the 8
 * stream budget (a village bed is one stream cap, not a per-tick cost).
 */
public final class VillageBed {
	/** The maximum simultaneous village slots (matches {@code FoleyBeds.MOST}). */
	public static final int MAX_BEDS = 2;
	private final NativeBridge bridge;
	private final Map<BlockPos, Long> bellRings = new HashMap<>();
	private final Map<BlockPos, Long> murmurs = new HashMap<>();
	private final Map<BlockPos, Long> animals = new HashMap<>();
	private final Set<BlockPos> seen = new HashSet<>();

	public VillageBed(NativeBridge bridge) {
		this.bridge = bridge;
	}

	/** Records {@code pos} as seen this scan; call after all scans to prune. */
	void see(BlockPos pos) {
		seen.add(new BlockPos(pos.getX(), pos.getY(), pos.getZ()));
	}

	/** Prunes slots whose bell was outside the radius last scan. */
	void prune() {
		bellRings.keySet().retainAll(seen);
		murmurs.keySet().retainAll(seen);
		animals.keySet().retainAll(seen);
		seen.clear();
	}

	/** How many slots are active. */
	public int size() {
		return bellRings.size();
	}

	/** True when the bell should ring now (rate limit). Records that it has. */
	boolean bellShouldRing(BlockPos bellPos, long nowTick, long minGapTicks) {
		BlockPos key = new BlockPos(bellPos.getX(), bellPos.getY(), bellPos.getZ());
		Long last = bellRings.get(key);
		if (last != null && nowTick - last < minGapTicks) return false;
		bellRings.put(key, nowTick);
		return true;
	}

	/** True when the murmur should fire now. */
	boolean murmurShouldFire(BlockPos bellPos, long nowTick, long minGapTicks) {
		BlockPos key = new BlockPos(bellPos.getX(), bellPos.getY(), bellPos.getZ());
		Long last = murmurs.get(key);
		if (last != null && nowTick - last < minGapTicks) return false;
		murmurs.put(key, nowTick);
		return true;
	}

	/** True when an animal scatter should fire now (per coop, not per bell). */
	boolean animalShouldFire(BlockPos here, long nowTick, long minGapTicks) {
		BlockPos key = new BlockPos(here.getX(), here.getY(), here.getZ());
		Long last = animals.get(key);
		if (last != null && nowTick - last < minGapTicks) return false;
		animals.put(key, nowTick);
		return true;
	}

	/** Plays a pre-rendered PCM buffer as a one-shot, like the M21 {@code playBuffer}. */
	void playBuffer(net.minecraft.resources.Identifier event, short @Nullable [] pcm, double x, double y, double z,
			float volume) {
		if (pcm == null) return;
		Minecraft mc = Minecraft.getInstance();
		var sound = new BufferSound(event, volume, x, y, z, pcm);
		mc.getSoundManager().play(sound);
	}

	/** The native bridge, so the controller can render one-shots. */
	NativeBridge bridge() {
		return bridge;
	}

	/** Whether to admit a new bell this tick, capped at {@link #MAX_BEDS}. */
	boolean admitBell(Minecraft minecraft, BlockPos bellPos) {
		BlockPos key = new BlockPos(bellPos.getX(), bellPos.getY(), bellPos.getZ());
		if (bellRings.containsKey(key)) return true;
		if (bellRings.size() >= MAX_BEDS) return false;
		if (minecraft.player == null) return false;
		double d = Math.sqrt(minecraft.player.blockPosition().distSqr(bellPos));
		// Prefer nearer bells when full.
		return true;
	}
}
