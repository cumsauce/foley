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

/**
 * The moving water, fire and lava near the listener (M2), from the blocks within {@value #RADIUS} blocks across and
 * {@value #DEPTH} up and down. Chunk sections with none of them are skipped whole.
 *
 * <p>Each source block counts by what it does: water flowing at its surface (still water is silent), a waterfall where
 * it lands (more) and down its face (a little), lava open to the air, fire, a lit campfire. Each also gets where it is
 * heard from and how loud: its power falls as 1/d², so the bed's gain is 2/d_eff for the power-weighted distance
 * d_eff = √(Σw / Σ(w/d²)), capped at 1 within 2 blocks, and it plays from the power-weighted centre.
 */
public record Surroundings(Water water, Fire fire, Lava lava) {
	static final int RADIUS = 12;
	static final int DEPTH = 8;
	public static final Surroundings NONE = new Surroundings(Water.STILL, Fire.OUT, Lava.NONE);
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
		Tally water = new Tally();
		Tally falls = new Tally();
		Tally fire = new Tally();
		Tally lava = new Tally();
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
								FluidState fluid = state.getFluidState();
								if (fluid.getType().isSame(Fluids.WATER)) {
									listenToWater(level, pos, fluid, d2, water, falls);
								} else if (fluid.getType().isSame(Fluids.LAVA)) {
									if (!level.getFluidState(pos.above()).getType().isSame(Fluids.LAVA)) {
										lava.add(1, d2, x, y, z);
									}
								} else if (state.is(BlockTags.FIRE)) {
									fire.add(1, d2, x, y, z);
								} else if (state.getValueOrElse(CampfireBlock.LIT, false)) {
									fire.add(0.6f, d2, x, y, z);
								}
							}
						}
					}
				}
			}
		}
		Tally both = water.plus(falls);
		return new Surroundings(
				water.total + falls.total > 0 ? new Water(water.total, falls.total, both.gain(), both.centre())
						: Water.STILL,
				fire.total > 0 ? new Fire(fire.total, fire.gain(), fire.centre()) : Fire.OUT,
				lava.total > 0 ? new Lava(lava.total, lava.gain(), lava.centre()) : Lava.NONE);
	}

	/** Flowing water at its surface babbles; a waterfall roars where it lands, and a little down its face. */
	private static void listenToWater(ClientLevel level, BlockPos.MutableBlockPos pos, FluidState fluid, double d2,
			Tally flow, Tally falls) {
		if (fluid.isSource()) return;
		int x = pos.getX(), y = pos.getY(), z = pos.getZ();
		if (fluid.getValueOrElse(FlowingFluid.FALLING, false)) {
			FluidState below = level.getFluidState(pos.setY(y - 1));
			pos.setY(y);
			boolean lands = !below.getType().isSame(Fluids.WATER) || !below.getValueOrElse(FlowingFluid.FALLING, false);
			falls.add(lands ? 1 : 0.1f, d2, x, y, z);
		} else if (!level.getFluidState(pos.setY(y + 1)).getType().isSame(Fluids.WATER)) {
			flow.add(1, d2, x, y, z);
		}
		pos.setY(y);
	}

	/** Sums of a kind of source: how much, its power at the listener, and where that power comes from. */
	private static final class Tally {
		float total;
		double power;
		double px, py, pz;

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
