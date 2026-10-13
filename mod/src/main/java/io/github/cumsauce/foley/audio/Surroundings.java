package io.github.cumsauce.foley.audio;

import java.util.Locale;
import java.util.function.Predicate;
import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.core.BlockPos;
import net.minecraft.tags.BlockTags;
import net.minecraft.util.Mth;
import net.minecraft.world.level.block.CampfireBlock;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.chunk.LevelChunk;
import net.minecraft.world.level.chunk.LevelChunkSection;
import net.minecraft.world.level.material.FlowingFluid;
import net.minecraft.world.level.material.FluidState;
import net.minecraft.world.level.material.Fluids;
import net.minecraft.world.phys.Vec3;
import org.jspecify.annotations.Nullable;

/**
 * The moving water, fire and lava near the listener (M2), from the blocks within {@value #RADIUS} blocks across and
 * {@value #DEPTH} up and down. Chunk sections with none of them are skipped whole.
 *
 * <p>Each source block counts by what it does: water flowing at its surface (still water is silent), a waterfall where
 * it lands (more) and down its face (a little), lava open to the air, fire, a lit campfire. Each also gets where it is
 * heard from and how loud: its power falls as 1/d², so the bed's gain is 2/d_eff for the power-weighted distance
 * d_eff = √(Σw / Σ(w/d²)), capped at 1 within 2 blocks, and it plays from the power-weighted centre.
 *
 * <p>A river or a burning forest is wide, not a point: each kind is also tallied in the four quarters round the
 * listener ({@code parts}, null where a quarter has too little to matter), each played as a bed of its own, so it
 * comes from all the directions it really does and each part is muffled by what's between it and the ear (several
 * point sources spread over the extent, as in multiple-direction amplitude panning, Pulkki 1999).
 *
 * @param water      all the moving water, together
 * @param waterParts the same in each quarter round the listener
 */
public record Surroundings(Water water, Fire fire, Lava lava, @Nullable Water[] waterParts,
		@Nullable Fire[] fireParts, @Nullable Lava[] lavaParts) {
	/** The quarters round the listener: +x or -x, +z or -z. */
	public static final int PARTS = 4;
	/** A quarter with less than this share of the strongest quarter's power joins it. */
	private static final double MINOR = 0.15;
	static final int RADIUS = 12;
	static final int DEPTH = 8;
	public static final Surroundings NONE = new Surroundings(Water.STILL, Fire.OUT, Lava.NONE, new Water[PARTS],
			new Fire[PARTS], new Lava[PARTS]);
	private static final Predicate<BlockState> HEARD = state -> !state.getFluidState().isEmpty()
			|| state.is(BlockTags.FIRE) || state.is(BlockTags.CAMPFIRES);

	/** Water: blocks flowing at the surface, blocks where a waterfall lands. */
	public record Water(float flow, float falls, float gain, Vec3 at) implements Scene {
		public static final Water STILL = new Water(0, 0, 0, Vec3.ZERO);

		public boolean heard() {
			return flow + falls > 0;
		}

		@Override
		public float[] params() {
			return new float[] {flow, falls, gain};
		}

		@Override
		public String toString() {
			return String.format(Locale.ROOT, "Water[flow=%.1f, falls=%.1f, gain=%.2f]", flow, falls, gain);
		}
	}

	/** Fire: blocks burning. */
	public record Fire(float size, float gain, Vec3 at) implements Scene {
		public static final Fire OUT = new Fire(0, 0, Vec3.ZERO);

		public boolean heard() {
			return size > 0;
		}

		@Override
		public float[] params() {
			return new float[] {size, gain};
		}

		@Override
		public String toString() {
			return String.format(Locale.ROOT, "Fire[size=%.1f, gain=%.2f]", size, gain);
		}
	}

	/** Lava: blocks open to the air. */
	public record Lava(float area, float gain, Vec3 at) implements Scene {
		public static final Lava NONE = new Lava(0, 0, Vec3.ZERO);

		public boolean heard() {
			return area > 0;
		}

		@Override
		public float[] params() {
			return new float[] {area, gain};
		}

		@Override
		public String toString() {
			return String.format(Locale.ROOT, "Lava[area=%.1f, gain=%.2f]", area, gain);
		}
	}

	/** Whether {@code at} is near enough to the listener for Foley to hear it, so it replaces the game's sound there. */
	public static boolean within(Vec3 listener, double x, double y, double z) {
		return Math.abs(x - listener.x) <= RADIUS + 1 && Math.abs(z - listener.z) <= RADIUS + 1
				&& Math.abs(y - listener.y) <= DEPTH + 1;
	}

	public static Surroundings sample(ClientLevel level, Vec3 eye) {
		// Index 0 of each: all of it; 1 to 4: each quarter round the listener.
		Tally[] water = Tally.parts();
		Tally[] falls = Tally.parts();
		Tally[] fire = Tally.parts();
		Tally[] lava = Tally.parts();
		int ex = Mth.floor(eye.x);
		int ey = Mth.floor(eye.y);
		int ez = Mth.floor(eye.z);
		int minY = Math.max(ey - DEPTH, level.getMinY());
		int maxY = Math.min(ey + DEPTH, level.getMaxY());
		BlockPos.MutableBlockPos pos = new BlockPos.MutableBlockPos();
		for (int cx = (ex - RADIUS) >> 4; cx <= (ex + RADIUS) >> 4; cx++) {
			for (int cz = (ez - RADIUS) >> 4; cz <= (ez + RADIUS) >> 4; cz++) {
				LevelChunk chunk = level.getChunk(cx, cz);
				for (int sy = minY >> 4; sy <= maxY >> 4; sy++) {
					LevelChunkSection section = chunk.getSection(level.getSectionIndexFromSectionY(sy));
					if (section.hasOnlyAir() || !section.maybeHas(HEARD)) continue;
					int x0 = Math.max(cx << 4, ex - RADIUS), x1 = Math.min((cx << 4) + 15, ex + RADIUS);
					int z0 = Math.max(cz << 4, ez - RADIUS), z1 = Math.min((cz << 4) + 15, ez + RADIUS);
					int y0 = Math.max(sy << 4, minY), y1 = Math.min((sy << 4) + 15, maxY);
					for (int x = x0; x <= x1; x++) {
						for (int y = y0; y <= y1; y++) {
							for (int z = z0; z <= z1; z++) {
								BlockState state = section.getBlockState(x & 15, y & 15, z & 15);
								if (!HEARD.test(state)) continue;
								pos.set(x, y, z);
								double d2 = Math.max(1, eye.distanceToSqr(x + 0.5, y + 0.5, z + 0.5));
								int part = 1 + (x + 0.5 >= eye.x ? 1 : 0) + (z + 0.5 >= eye.z ? 2 : 0);
								FluidState fluid = state.getFluidState();
								if (fluid.getType().isSame(Fluids.WATER)) {
									listenToWater(level, pos, fluid, d2, water, falls, part);
								} else if (fluid.getType().isSame(Fluids.LAVA)) {
									if (!level.getFluidState(pos.above()).getType().isSame(Fluids.LAVA)) {
										Tally.add(lava, part, 1, d2, x, y, z);
									}
								} else if (state.is(BlockTags.FIRE)) {
									Tally.add(fire, part, 1, d2, x, y, z);
								} else if (state.getValueOrElse(CampfireBlock.LIT, false)) {
									Tally.add(fire, part, 0.6f, d2, x, y, z);
								}
							}
						}
					}
				}
			}
		}
		Water[] waterParts = new Water[PARTS];
		Fire[] fireParts = new Fire[PARTS];
		Lava[] lavaParts = new Lava[PARTS];
		Tally[] wet = new Tally[PARTS + 1];
		for (int i = 0; i <= PARTS; i++) wet[i] = water[i].plus(falls[i]);
		int[] waterInto = Tally.merged(wet);
		int[] fireInto = Tally.merged(fire);
		int[] lavaInto = Tally.merged(lava);
		for (int i = 1; i <= PARTS; i++) {
			// A minor quarter's sources join the strongest.
			water[waterInto[i]] = water[waterInto[i]].plus(i == waterInto[i] ? new Tally() : water[i]);
			falls[waterInto[i]] = falls[waterInto[i]].plus(i == waterInto[i] ? new Tally() : falls[i]);
			wet[waterInto[i]] = i == waterInto[i] ? wet[i] : wet[waterInto[i]].plus(wet[i]);
			fire[fireInto[i]] = i == fireInto[i] ? fire[i] : fire[fireInto[i]].plus(fire[i]);
			lava[lavaInto[i]] = i == lavaInto[i] ? lava[i] : lava[lavaInto[i]].plus(lava[i]);
		}
		for (int i = 1; i <= PARTS; i++) {
			if (waterInto[i] == i) waterParts[i - 1] = water(water[i], falls[i], wet[i]);
			if (fireInto[i] == i && fire[i].total > 0) {
				fireParts[i - 1] = new Fire(fire[i].total, fire[i].gain(), fire[i].centre());
			}
			if (lavaInto[i] == i && lava[i].total > 0) {
				lavaParts[i - 1] = new Lava(lava[i].total, lava[i].gain(), lava[i].centre());
			}
		}
		return new Surroundings(water(water[0], falls[0], wet[0]),
				fire[0].total > 0 ? new Fire(fire[0].total, fire[0].gain(), fire[0].centre()) : Fire.OUT,
				lava[0].total > 0 ? new Lava(lava[0].total, lava[0].gain(), lava[0].centre()) : Lava.NONE,
				waterParts, fireParts, lavaParts);
	}

	private static Water water(Tally flow, Tally falls, Tally both) {
		return flow.total + falls.total > 0 ? new Water(flow.total, falls.total, both.gain(), both.centre())
				: Water.STILL;
	}

	/** Flowing water at its surface babbles; a waterfall roars where it lands, and a little down its face. */
	private static void listenToWater(ClientLevel level, BlockPos.MutableBlockPos pos, FluidState fluid, double d2,
			Tally[] flow, Tally[] falls, int part) {
		if (fluid.isSource()) return;
		int x = pos.getX(), y = pos.getY(), z = pos.getZ();
		if (fluid.getValueOrElse(FlowingFluid.FALLING, false)) {
			FluidState below = level.getFluidState(pos.setY(y - 1));
			pos.setY(y);
			boolean lands = !below.getType().isSame(Fluids.WATER) || !below.getValueOrElse(FlowingFluid.FALLING, false);
			Tally.add(falls, part, lands ? 1 : 0.1f, d2, x, y, z);
		} else if (!level.getFluidState(pos.setY(y + 1)).getType().isSame(Fluids.WATER)) {
			Tally.add(flow, part, 1, d2, x, y, z);
		}
		pos.setY(y);
	}

	/** Sums of a kind of source: how much, its power at the listener, and where that power comes from. */
	private static final class Tally {
		float total;
		double power;
		double px, py, pz;

		/** A fresh tally for all of a kind and one for each quarter. */
		static Tally[] parts() {
			Tally[] parts = new Tally[PARTS + 1];
			for (int i = 0; i < parts.length; i++) parts[i] = new Tally();
			return parts;
		}

		/** Adds to the whole and to quarter {@code part}. */
		static void add(Tally[] tallies, int part, float amount, double d2, int x, int y, int z) {
			tallies[0].add(amount, d2, x, y, z);
			tallies[part].add(amount, d2, x, y, z);
		}

		/**
		 * For each quarter (1 to 4), the quarter it is heard as part of: itself, or the strongest when it has less than
		 * {@value #MINOR} of that one's power. Index 0 is unused.
		 */
		static int[] merged(Tally[] tallies) {
			int strongest = 1;
			for (int i = 2; i <= PARTS; i++) {
				if (tallies[i].power > tallies[strongest].power) strongest = i;
			}
			int[] into = new int[PARTS + 1];
			for (int i = 1; i <= PARTS; i++) {
				into[i] = tallies[i].total > 0 && tallies[i].power >= MINOR * tallies[strongest].power ? i : strongest;
			}
			return into;
		}

		void add(float amount, double d2, int x, int y, int z) {
			total += amount;
			double p = amount / d2;
			power += p;
			px += p * (x + 0.5);
			py += p * (y + 0.5);
			pz += p * (z + 0.5);
		}

		Tally plus(Tally other) {
			Tally sum = new Tally();
			sum.total = total + other.total;
			sum.power = power + other.power;
			sum.px = px + other.px;
			sum.py = py + other.py;
			sum.pz = pz + other.pz;
			return sum;
		}

		/** 2 / d_eff, at most 1. */
		float gain() {
			return total <= 0 ? 0 : (float) Math.min(1, 2 * Math.sqrt(power / total));
		}

		Vec3 centre() {
			return power <= 0 ? Vec3.ZERO : new Vec3(px / power, py / power, pz / power);
		}
	}
}
