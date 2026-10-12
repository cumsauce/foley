package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Holder;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.sounds.SoundSource;
import net.minecraft.tags.BlockTags;
import net.minecraft.util.RandomSource;
import net.minecraft.world.level.biome.Biome;
import net.minecraft.world.level.biome.Biomes;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.level.material.Fluids;
import net.minecraft.world.phys.Vec3;
import org.jspecify.annotations.Nullable;

/**
 * Ambient life (M5), layered on top of the game's sounds: each call a generated one-shot played where the animal is, so
 * it pans and Earshot shapes it.
 *
 * <ul>
 *   <li><b>Birds</b> sing from the trees round about by day, most at the dawn chorus, not in the rain. Each biome has
 *       its own three species, so a forest keeps its voices.</li>
 *   <li><b>Crickets</b> chirp from the grass at night when it's warm enough (from 10 °C), at Dolbear's rate for the
 *       temperature, each staying where it is.</li>
 *   <li><b>Frogs</b> call from still water at night or in the rain; bigger ones in swamps.</li>
 * </ul>
 *
 * Only in a dimension with a sky and no ceiling (the Overworld), and only when the listener is outside enough to hear
 * it (the wind's exposure).
 */
public final class Life {
	/** Species of bird, generated: {@code foley:bird_<n>}. */
	public static final int SPECIES = 12;
	/** Crickets are rendered for these temperatures (°C): {@code foley:crickets_<n>}. */
	public static final float[] CRICKET_CELSIUS = {12, 16, 20, 24, 28};
	/** Frogs' body lengths, cm: {@code foley:frog_<n>}. */
	public static final float[] FROG_CM = {3, 7, 14};
	private static final int MAX_CRICKETS = 4;

	private final RandomSource random = RandomSource.create();
	private final List<Cricket> crickets = new ArrayList<>();
	private int ticks;
	private int calls;

	/** A cricket in the grass, and when it next chirps (tick). */
	private record Cricket(BlockPos at, int next) {}

	/** Calls made so far. For the client gametest. */
	public int calls() {
		return calls;
	}

	public void tick(Minecraft minecraft) {
		ClientLevel level = minecraft.level;
		if (level == null || minecraft.player == null || !level.dimensionType().hasSkyLight()
				|| level.dimensionType().hasCeiling()) {
			crickets.clear();
			return;
		}
		ticks++;
		var wind = FoleyClient.wind();
		float exposure = wind == null ? 1 : wind.scene().exposure();
		if (exposure < 0.2f) return;
		Vec3 eye = minecraft.player.getEyePosition();
		long time = Math.floorMod(level.getOverworldClockTime(), 24000L);
		boolean night = time >= 13000 && time < 23000;
		boolean dawn = time >= 23000 || time < 2000;
		boolean raining = level.isRaining();
		Holder<Biome> biome = level.getBiome(BlockPos.containing(eye));
		float celsius = 20 * biome.value().getBaseTemperature() - (night ? 5 : 0);

		if (ticks % 20 == 0) {
			// Birds: about one song every 4 s, three times as many at dawn.
			if (!night && !raining && random.nextFloat() < (dawn ? 0.75f : 0.25f) * exposure) {
				BlockPos tree = find(level, eye, 24, state -> state.is(BlockTags.LEAVES));
				if (tree != null) {
					int species = Math.floorMod(biome.unwrapKey().map(k -> k.identifier().hashCode()).orElse(0)
							+ 5 * random.nextInt(3), SPECIES);
					call(minecraft, "bird_" + species, tree.above(), 0.8f);
				}
			}
			// Frogs: at night or in the rain, from still water.
			boolean swamp = biome.is(Biomes.SWAMP) || biome.is(Biomes.MANGROVE_SWAMP);
			if ((night || raining) && celsius >= 8 && random.nextFloat() < (swamp ? 0.6f : 0.15f)) {
				BlockPos water = find(level, eye, 20, state -> state.getFluidState().isSourceOfType(Fluids.WATER));
				if (water != null) {
					int size = swamp ? 1 + random.nextInt(2) : random.nextInt(2);
					call(minecraft, "frog_" + size, water.above(), 0.6f);
				}
			}
		}
		// Crickets: a few, each in its place, chirping on and on.
		if (!night || raining || celsius < 10) {
			crickets.clear();
			return;
		}
		crickets.removeIf(c -> c.at.distToCenterSqr(eye) > 24 * 24);
		if (crickets.size() < MAX_CRICKETS && ticks % 20 == 0) {
			BlockPos grass = find(level, eye, 14, state -> state.is(Blocks.GRASS_BLOCK) || state.is(Blocks.SHORT_GRASS)
					|| state.is(Blocks.TALL_GRASS));
			if (grass != null) crickets.add(new Cricket(grass.above(), ticks + random.nextInt(60)));
		}
		int band = 0;
		for (int i = 1; i < CRICKET_CELSIUS.length; i++) {
			if (Math.abs(CRICKET_CELSIUS[i] - celsius) < Math.abs(CRICKET_CELSIUS[band] - celsius)) band = i;
		}
		for (int i = 0; i < crickets.size(); i++) {
			Cricket cricket = crickets.get(i);
			if (ticks < cricket.next) continue;
			call(minecraft, "crickets_" + band, cricket.at, 0.5f);
			// Each rendition is 3 s.
			crickets.set(i, new Cricket(cricket.at, ticks + 60));
		}
	}

	private void call(Minecraft minecraft, String name, BlockPos at, float volume) {
		SoundEvent event = SoundEvent.createVariableRangeEvent(Identifier.fromNamespaceAndPath("foley", name));
		minecraft.getSoundManager().play(new SimpleSoundInstance(event, SoundSource.AMBIENT, volume, 1f,
				SoundInstance.createUnseededRandom(), at.getX() + 0.5, at.getY() + 0.2, at.getZ() + 0.5));
		calls++;
	}

	/** The top block of one of a few random columns within {@code radius} that {@code wanted} accepts, or null. */
	private @Nullable BlockPos find(ClientLevel level, Vec3 eye, int radius,
			java.util.function.Predicate<BlockState> wanted) {
		for (int tries = 0; tries < 8; tries++) {
			int x = (int) Math.floor(eye.x) + random.nextInt(2 * radius + 1) - radius;
			int z = (int) Math.floor(eye.z) + random.nextInt(2 * radius + 1) - radius;
			BlockPos top = new BlockPos(x, level.getHeight(Heightmap.Types.MOTION_BLOCKING, x, z) - 1, z);
			if (Math.abs(top.getY() - eye.y) > 16) continue;
			if (wanted.test(level.getBlockState(top))) return top;
			// Grass and flowers sit on top of the ground, under the motion-blocking height.
			BlockPos above = top.above();
			if (wanted.test(level.getBlockState(above))) return above;
		}
		return null;
	}
}
