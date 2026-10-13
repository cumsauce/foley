package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
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
import net.minecraft.tags.BiomeTags;
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
 *   <li><b>Birds</b> sing from the trees round about by day, most at the dawn chorus, not in the rain: real ones, each
 *       where it lives (robins, cardinals and chickadees in the woods, chickadees in the taiga, doves and crows in the
 *       open). A bird stays on its perch and sings again for a while before another takes over.</li>
 *   <li><b>Crickets</b> chirp from the grass at night when it's warm enough (from 10 °C), at Dolbear's rate for the
 *       temperature, each staying where it is.</li>
 *   <li><b>Frogs</b> call from still water at night or in the rain; bigger ones in swamps.</li>
 *   <li><b>Surf</b> breaks on the shore near the sea, a wave every 6 to 12 s, bigger in rain and storms; a lake
 *       laps when the wind blows.</li>
 *   <li><b>Woodpeckers</b> drum in woods by day; <b>owls</b> hoot in woods and taiga at night.</li>
 *   <li><b>Cicadas</b> drone in the heat of the day, in jungle, savanna and badlands.</li>
 *   <li><b>Wolves</b> howl now and then on cold nights, far off.</li>
 *   <li>After rain, the trees go on <b>dripping</b> for a minute.</li>
 * </ul>
 *
 * Only in a dimension with a sky and no ceiling (the Overworld), and only when the listener is outside enough to hear
 * it: out in the open, or under trees (the wind's exposure and leaves).
 */
public final class Life {
	/** Birds, generated: {@code foley:bird_<n>}, in the native order. */
	public static final int CROW = 0, DOVE = 1, ROBIN = 2, CARDINAL = 3, CHICKADEE = 4;
	public static final int SPECIES = 5;
	/** How loud each sings: a crow carries, a dove is soft. */
	private static final float[] BIRD_VOLUME = {0.7f, 0.4f, 0.35f, 0.4f, 0.3f};
	/** Crickets are rendered for these temperatures (°C): {@code foley:crickets_<n>}. */
	public static final float[] CRICKET_CELSIUS = {12, 16, 20, 24, 28};
	/** Frogs' body lengths, cm: {@code foley:frog_<n>}. */
	public static final float[] FROG_CM = {3, 7, 14};
	private static final int MAX_CRICKETS = 4;
	/** Waves are rendered for these heights (m): a lake's lapping, a calm sea, a rough one. */
	public static final float[] SURF_M = {0.3f, 1.0f, 2.2f};
	/** Cicadas are rendered for these temperatures (°C). */
	public static final float[] CICADA_CELSIUS = {26, 32, 38};
	/** How long the trees drip after rain stops, ticks. */
	private static final int DRIPPING = 1200;

	private final RandomSource random = RandomSource.create();
	private final List<Cricket> crickets = new ArrayList<>();
	private int ticks;
	private int calls;
	private int nextWave;
	private int nextCicadas;
	private int rainedAt = Integer.MIN_VALUE / 2;
	private @Nullable BlockPos shore;
	private int shoreFoundAt = Integer.MIN_VALUE / 2;
	/** The bird singing now, where, and until when (tick) it keeps at it. */
	private @Nullable BlockPos perch;
	private int singer;
	private int perchUntil;

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
		// Rain counts wherever the listener sat it out, indoors too.
		if (level.isRaining()) rainedAt = ticks;
		var wind = FoleyClient.wind();
		// Under trees the sky is hidden, but the woods are all around: they count as outside.
		float exposure = wind == null ? 1
				: Math.max(wind.scene().exposure(), 0.8f * wind.scene().leaves());
		if (exposure < 0.2f) return;
		Vec3 eye = minecraft.player.getEyePosition();
		long time = Math.floorMod(level.getOverworldClockTime(), 24000L);
		boolean night = time >= 13000 && time < 23000;
		boolean dawn = time >= 23000 || time < 2000;
		boolean raining = level.isRaining();
		Holder<Biome> biome = level.getBiome(BlockPos.containing(eye));
		float celsius = 20 * biome.value().getBaseTemperature() - (night ? 5 : 0);

		if (ticks % 20 == 0) {
			// Birds: a song every half a minute or so, four times as often at dawn. Mostly the same bird again from
			// its perch, as one sings on and off for a minute or two.
			if (FoleyClient.wants(FoleyConfig::birds) && !night && !raining && random.nextFloat() < (dawn ? 0.12f : 0.03f) * exposure) {
				if (perch == null || ticks > perchUntil || perch.distSqr(BlockPos.containing(eye)) > 48 * 48
						|| random.nextFloat() < 0.3f) {
					singer = pickBird(biome, celsius);
					perch = find(level, eye, singer == CROW ? 40 : 24, state -> state.is(BlockTags.LEAVES));
					perchUntil = ticks + 20 * (40 + random.nextInt(80));
				}
				if (perch != null) call(minecraft, "bird_" + singer, perch.above(), BIRD_VOLUME[singer]);
			}
			// Frogs: at night or in the rain, from still water.
			boolean swamp = biome.is(Biomes.SWAMP) || biome.is(Biomes.MANGROVE_SWAMP);
			if (FoleyClient.wants(FoleyConfig::frogs) && (night || raining) && celsius >= 8 && random.nextFloat() < (swamp ? 0.6f : 0.15f)) {
				BlockPos water = find(level, eye, 20, state -> state.getFluidState().isSourceOfType(Fluids.WATER));
				if (water != null) {
					int size = swamp ? 1 + random.nextInt(2) : random.nextInt(2);
					call(minecraft, "frog_" + size, water.above(), 0.6f);
				}
			}
		}
		if (ticks % 20 == 0) {
			boolean woods = biome.is(BiomeTags.IS_FOREST) || biome.is(BiomeTags.IS_TAIGA) || biome.is(BiomeTags.IS_JUNGLE);
			// A woodpecker every minute or so in the woods by day.
			if (FoleyClient.wants(FoleyConfig::birds) && woods && !night && !raining && random.nextFloat() < 0.02f) {
				BlockPos tree = find(level, eye, 24, state -> state.is(BlockTags.LEAVES));
				if (tree != null) call(minecraft, "woodpecker_0", tree.below(2), 0.6f);
			}
			// Owls in the woods at night, from farther off.
			if (FoleyClient.wants(FoleyConfig::birds) && night && !raining
					&& (biome.is(BiomeTags.IS_FOREST) || biome.is(BiomeTags.IS_TAIGA)) && random.nextFloat() < 0.03f) {
				BlockPos tree = find(level, eye, 40, state -> state.is(BlockTags.LEAVES));
				if (tree != null) call(minecraft, "owl_0", tree, 0.7f);
			}
			// Now and then, a wolf howls far away on a cold night.
			boolean cold = biome.is(BiomeTags.IS_TAIGA) || celsius < 6;
			if (FoleyClient.wants(FoleyConfig::birds) && night && cold && !raining && random.nextFloat() < 0.004f) {
				double angle = random.nextDouble() * Math.PI * 2;
				int x = (int) (eye.x + 50 * Math.cos(angle));
				int z = (int) (eye.z + 50 * Math.sin(angle));
				int y = level.getHeight(Heightmap.Types.MOTION_BLOCKING, x, z);
				call(minecraft, "howl_0", new BlockPos(x, Math.max(y, (int) eye.y - 8), z), 0.8f);
			}
		}
		// Surf: a wave every 6 to 12 s on a shore near the sea, or a lake lapping in the wind.
		if (FoleyClient.wants(FoleyConfig::water) && ticks >= nextWave) {
			nextWave = ticks + 120 + random.nextInt(120);
			if (ticks - shoreFoundAt > 200 || shore == null) {
				shore = findShore(level, eye, 32);
				shoreFoundAt = ticks;
			}
			boolean sea = biome.is(BiomeTags.IS_OCEAN) || biome.is(BiomeTags.IS_DEEP_OCEAN)
					|| biome.is(BiomeTags.IS_BEACH);
			float windSpeed = wind == null ? 0 : wind.scene().speed();
			if (shore != null && (sea || windSpeed > 4)) {
				int size = !sea ? 0 : level.isThundering() || raining ? 2 : 1;
				call(minecraft, "surf_" + size, shore, sea ? 1f : 0.6f);
			}
		}
		// Cicadas through the heat of the day: one chorus after another.
		boolean hot = biome.is(BiomeTags.IS_JUNGLE) || biome.is(BiomeTags.IS_SAVANNA) || biome.is(BiomeTags.IS_BADLANDS);
		if (FoleyClient.wants(FoleyConfig::crickets) && hot && !night && !raining && ticks >= nextCicadas) {
			nextCicadas = ticks + 110 + random.nextInt(40);
			BlockPos tree = find(level, eye, 20, state -> state.is(BlockTags.LEAVES));
			if (tree != null) {
				float warm = Math.max(celsius, 28);
				int band = 0;
				for (int i = 1; i < CICADA_CELSIUS.length; i++) {
					if (Math.abs(CICADA_CELSIUS[i] - warm) < Math.abs(CICADA_CELSIUS[band] - warm)) band = i;
				}
				call(minecraft, "cicadas_" + band, tree, 0.45f);
			}
		}
		// After rain, the leaves drip for a while: often at first, then now and then. Not where it never rains, or
		// where it snowed.
		int since = ticks - rainedAt;
		boolean wet = biome.value().hasPrecipitation() && celsius > 3;
		if (FoleyClient.wants(FoleyConfig::drips) && wet && !raining && since < DRIPPING
				&& random.nextFloat() < 0.4f * (1 - since / (float) DRIPPING)) {
			BlockPos leaf = find(level, eye, 8, state -> state.is(BlockTags.LEAVES));
			if (leaf != null) {
				int ground = level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES, leaf.getX(), leaf.getZ());
				dripAt(minecraft, new BlockPos(leaf.getX(), ground, leaf.getZ()));
			}
		}
		// Crickets: a few, each in its place, chirping on and on.
		if (!FoleyClient.wants(FoleyConfig::crickets) || !night || raining || celsius < 10) {
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

	/** A bird that lives here, more or less likely by what the biome is like. */
	private int pickBird(Holder<Biome> biome, float celsius) {
		int[] weights; // crow, dove, robin, cardinal, chickadee
		if (biome.is(BiomeTags.IS_TAIGA) || celsius < 6) {
			weights = new int[] {35, 0, 20, 0, 45};
		} else if (biome.is(BiomeTags.IS_FOREST)) {
			weights = new int[] {15, 10, 30, 25, 20};
		} else if (biome.is(BiomeTags.IS_JUNGLE) || biome.is(Biomes.SWAMP) || biome.is(Biomes.MANGROVE_SWAMP)) {
			weights = new int[] {35, 30, 0, 35, 0};
		} else {
			weights = new int[] {30, 35, 25, 10, 0};
		}
		int roll = random.nextInt(100);
		for (int i = 0; i < weights.length; i++) {
			roll -= weights[i];
			if (roll < 0) return i;
		}
		return CROW;
	}

	private void call(Minecraft minecraft, String name, BlockPos at, float volume) {
		if (!FoleyClient.oneShot()) return;
		SoundEvent event = SoundEvent.createVariableRangeEvent(Identifier.fromNamespaceAndPath("foley", name));
		minecraft.getSoundManager().play(new SimpleSoundInstance(event, SoundSource.AMBIENT, volume * FoleyClient.gain(), 1f,
				SoundInstance.createUnseededRandom(), at.getX() + 0.5, at.getY() + 0.2, at.getZ() + 0.5));
		calls++;
	}

	/** A drop falling from the leaves onto what's under them. */
	private void dripAt(Minecraft minecraft, BlockPos at) {
		if (!FoleyClient.oneShot()) return;
		float volume = (0.15f + 0.25f * random.nextFloat()) * FoleyClient.gain();
		minecraft.getSoundManager().play(new SimpleSoundInstance(Replacements.DRIP_STONE, SoundSource.AMBIENT, volume,
				0.9f + 0.2f * random.nextFloat(), SoundInstance.createUnseededRandom(), at.getX() + random.nextFloat(),
				at.getY() + 0.05, at.getZ() + random.nextFloat()));
	}

	/**
	 * Where open water meets land within {@code radius}: a column whose top is a water source with land a couple of
	 * blocks off one side, or null.
	 */
	private @Nullable BlockPos findShore(ClientLevel level, Vec3 eye, int radius) {
		for (int tries = 0; tries < 24; tries++) {
			int x = (int) Math.floor(eye.x) + random.nextInt(2 * radius + 1) - radius;
			int z = (int) Math.floor(eye.z) + random.nextInt(2 * radius + 1) - radius;
			int top = level.getHeight(Heightmap.Types.MOTION_BLOCKING, x, z) - 1;
			if (Math.abs(top - eye.y) > 16) continue;
			if (!level.getFluidState(new BlockPos(x, top, z)).isSourceOfType(Fluids.WATER)) continue;
			for (int[] d : new int[][] {{2, 0}, {-2, 0}, {0, 2}, {0, -2}}) {
				BlockPos land = new BlockPos(x + d[0], level.getHeight(Heightmap.Types.MOTION_BLOCKING, x + d[0], z + d[1]) - 1,
						z + d[1]);
				if (!level.getFluidState(land).getType().isSame(Fluids.WATER) && Math.abs(land.getY() - top) <= 2) {
					return new BlockPos(x, top, z);
				}
			}
		}
		return null;
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
