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
 *   <li>Frogs <b>answer</b> each other along the water; night insects come in through dusk.</li>
 *   <li>Lake <b>ice sings</b> in the cold, most at dawn and dusk; lakes lap in a light wind.</li>
 *   <li>In <b>caves</b>, a stone settles now and then off in the dark, and water drips from the roof.</li>
 * </ul>
 *
 * Only in a dimension with a sky and no ceiling (the Overworld), and only when the listener is outside enough to hear
 * it: out in the open, or under trees (the wind's exposure and leaves).
 */
public final class Life {
	/** Birds, generated: {@code foley:bird_<n>}, in the native order. */
	public static final int CROW = 0, DOVE = 1, ROBIN = 2, CARDINAL = 3, CHICKADEE = 4, GULL = 5, LOON = 6, JAY = 7,
			WARBLER = 8;
	public static final int SPECIES = 9;
	/** How loud each sings: a crow carries, a dove is soft. */
	// Crows and jays are harsh: rarer inland (see pickBird) and quieter, so a day in the woods stays easy.
	private static final float[] BIRD_VOLUME = {0.45f, 0.4f, 0.35f, 0.4f, 0.3f, 0.6f, 0.7f, 0.35f, 0.3f};
	/** When something loud (an explosion, thunder) last went off, System.nanoTime(). */
	private static volatile long startledAt = Long.MIN_VALUE / 2;
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
	/** A frog answering another: where, its size, and when (tick). */
	private @Nullable BlockPos frogReply;
	private int frogReplySize;
	private int frogReplyAt;
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

	/** The world all the places above are in. */
	private @Nullable ClientLevel placesIn;

	public void tick(Minecraft minecraft) {
		ClientLevel level = minecraft.level;
		if (level != placesIn) {
			// A new world or dimension: what was remembered is somewhere else.
			placesIn = level;
			crickets.clear();
			frogReply = null;
			perch = null;
			shore = null;
		}
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
		if (exposure < 0.2f) {
			caves(minecraft, level);
			return;
		}
		Vec3 eye = minecraft.player.getEyePosition();
		long time = Math.floorMod(level.getOverworldClockTime(), 24000L);
		boolean night = time >= 13000 && time < 23000;
		boolean dawn = time >= 23000 || time < 2000;
		boolean raining = level.isRaining();
		Holder<Biome> biome = level.getBiome(BlockPos.containing(eye));
		float celsius = 20 * biome.value().getBaseTemperature() - (night ? 5 : 0);

		if (ticks % 20 == 0) {
			// Birds: a song a minute or so through the day, several times as often in the dawn chorus, more again
			// in the evening; fewer in a snowy hush. Mostly the same bird again from its perch, as one sings on and off
			// for a minute or two. Something loud silences them all for a while.
			boolean startled = System.nanoTime() - startledAt < 30_000_000_000L;
			float snowy = celsius < 0.5f ? 0.4f : 1;
			if (FoleyClient.wants(FoleyConfig::birds) && !night && !raining && !startled
					&& random.nextFloat() < 0.12f * birdActivity(time) * snowy * exposure) {
				if (perch == null || ticks > perchUntil || perch.distSqr(BlockPos.containing(eye)) > 48 * 48
						|| random.nextFloat() < 0.3f) {
					singer = pickBird(biome, celsius);
					perch = singer == GULL
							? find(level, eye, 40, state -> state.getFluidState().isSourceOfType(Fluids.WATER))
							: find(level, eye, singer == CROW || singer == JAY ? 40 : 24,
									state -> state.is(BlockTags.LEAVES));
					perchUntil = ticks + 20 * (40 + random.nextInt(80));
				}
				if (perch != null) {
					call(minecraft, "bird_" + singer, singer == GULL ? perch.above(4) : perch.above(),
							BIRD_VOLUME[singer]);
				}
			}
			// A loon wails across a lake at dusk and through the night.
			if (FoleyClient.wants(FoleyConfig::birds) && (night || time >= 12000) && !raining && celsius > 0
					&& celsius < 16 && !biome.is(BiomeTags.IS_OCEAN) && random.nextFloat() < 0.012f) {
				BlockPos lake = find(level, eye, 48, state -> state.getFluidState().isSourceOfType(Fluids.WATER));
				if (lake != null) call(minecraft, "bird_" + LOON, lake.above(), BIRD_VOLUME[LOON]);
			}
			// Bees round flowers on a warm day; flies over a swamp.
			boolean swampy = biome.is(Biomes.SWAMP) || biome.is(Biomes.MANGROVE_SWAMP);
			if (FoleyClient.wants(FoleyConfig::crickets) && !night && !raining && celsius >= 12
					&& random.nextFloat() < 0.02f) {
				// Now and then, softly, and from a little way off: a buzz at the ear is the opposite of restful.
				BlockPos bloom = swampy ? find(level, eye, 12, state -> state.is(Blocks.GRASS_BLOCK)
						|| state.getFluidState().isSourceOfType(Fluids.WATER))
						: find(level, eye, 12, state -> state.is(BlockTags.FLOWERS));
				if (bloom != null) call(minecraft, swampy ? "buzz_1" : "buzz_0", bloom.above(), 0.2f);
			}
			// Frogs: at night or in the rain, from still water.
			boolean swamp = biome.is(Biomes.SWAMP) || biome.is(Biomes.MANGROVE_SWAMP);
			if (FoleyClient.wants(FoleyConfig::frogs) && (night || raining) && celsius >= 8 && random.nextFloat() < (swamp ? 0.6f : 0.15f)) {
				BlockPos water = find(level, eye, 20, state -> state.getFluidState().isSourceOfType(Fluids.WATER));
				if (water != null) {
					int size = swamp ? 1 + random.nextInt(2) : random.nextInt(2);
					call(minecraft, "frog_" + size, water.above(), 0.6f);
					// Often another answers from elsewhere along the water, a moment later.
					if (random.nextFloat() < 0.5f) {
						frogReply = water.offset(random.nextInt(13) - 6, 0, random.nextInt(13) - 6);
						frogReplySize = size;
						frogReplyAt = ticks + 10 + random.nextInt(30);
					}
				}
			}
		}
		// A bird the player runs at, or near something loud, takes off from its perch, and sings elsewhere later.
		if (perch != null && minecraft.player != null && FoleyClient.wants(FoleyConfig::birds) && singer != GULL) {
			double near = perch.distToCenterSqr(eye);
			boolean rushing = minecraft.player.getDeltaMovement().horizontalDistance() > 0.15 && near < 7 * 7;
			boolean scared = System.nanoTime() - startledAt < 1_000_000_000L && near < 24 * 24;
			if (rushing || scared) {
				call(minecraft, "flush_0", perch.above(), 0.5f);
				perch = null;
			}
		}
		if (frogReply != null && ticks >= frogReplyAt) {
			call(minecraft, "frog_" + frogReplySize, frogReply.above(), 0.5f);
			frogReply = null;
		}
		// Lake ice sings in the cold, most as the air warms or cools at dawn and dusk.
		if (FoleyClient.wants(FoleyConfig::water) && ticks % 20 == 0 && celsius < 2
				&& random.nextFloat() < (dawn || time >= 11500 && time < 13500 ? 0.06f : 0.015f)) {
			BlockPos ice = find(level, eye, 32, state -> state.is(Blocks.ICE));
			if (ice != null) call(minecraft, "ice_0", ice, 0.6f);
		}
		if (ticks % 20 == 0) {
			boolean woods = biome.is(BiomeTags.IS_FOREST) || biome.is(BiomeTags.IS_TAIGA) || biome.is(BiomeTags.IS_JUNGLE);
			// A woodpecker every two minutes or so in the woods by day, soft and off among the trees: its drumming is
			// sharp up close.
			if (FoleyClient.wants(FoleyConfig::birds) && woods && !night && !raining && random.nextFloat() < 0.008f) {
				BlockPos tree = find(level, eye, 32, state -> state.is(BlockTags.LEAVES));
				if (tree != null) call(minecraft, "woodpecker_0", tree.below(2), 0.35f);
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
			if (shore != null && (sea || windSpeed > 1.5f)) {
				int size = !sea ? 0 : level.isThundering() || raining ? 2 : 1;
				call(minecraft, "surf_" + size, shore, sea ? 1f : 0.6f);
			}
		}
		// Cicadas through the heat of the day: one chorus after another.
		boolean hot = biome.is(BiomeTags.IS_JUNGLE) || biome.is(BiomeTags.IS_SAVANNA) || biome.is(BiomeTags.IS_BADLANDS);
		// Cicadas sing in the heat of the afternoon (noon to 5 pm), and from the trees, not over everything.
		boolean afternoon = time >= 6000 && time < 11000;
		if (FoleyClient.wants(FoleyConfig::crickets) && hot && afternoon && !raining && ticks >= nextCicadas) {
			nextCicadas = ticks + 110 + random.nextInt(40);
			BlockPos tree = find(level, eye, 20, state -> state.is(BlockTags.LEAVES));
			if (tree != null) {
				float warm = Math.max(celsius, 28);
				int band = 0;
				for (int i = 1; i < CICADA_CELSIUS.length; i++) {
					if (Math.abs(CICADA_CELSIUS[i] - warm) < Math.abs(CICADA_CELSIUS[band] - warm)) band = i;
				}
				call(minecraft, "cicadas_" + band, tree, 0.3f);
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
		float insects = insectActivity(time);
		if (!FoleyClient.wants(FoleyConfig::crickets) || insects <= 0 || raining || celsius < 10) {
			crickets.clear();
			return;
		}
		crickets.removeIf(c -> c.at.distToCenterSqr(eye) > 24 * 24);
		if (crickets.size() < Math.round(MAX_CRICKETS * insects) && ticks % 20 == 0) {
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

	/** Notes a sound the game plays: something loud (an explosion, thunder) startles the birds. Any thread. */
	public static void heard(net.minecraft.client.resources.sounds.SoundInstance sound) {
		String path = sound.getIdentifier().getPath();
		if (path.contains("explode") || path.contains("thunder") || path.contains("firework_rocket.blast")) {
			startledAt = System.nanoTime();
		}
	}

	/**
	 * How much the birds sing at {@code time} (ticks, 0 at 6 am), 0 to 1: the dawn chorus before and round sunrise, a
	 * lull through the day, a smaller chorus in the evening.
	 */
	static float birdActivity(long time) {
		double hour = (time / 1000.0 + 6) % 24;
		double dawn = Math.exp(-Math.pow((hour - 5.5) / 1.2, 2));
		double evening = 0.5 * Math.exp(-Math.pow((hour - 18.3) / 0.9, 2));
		// Through the day, songs with long quiet between.
		double day = hour > 6 && hour < 19 ? 0.15 : 0;
		return (float) Math.min(1, dawn + evening + day);
	}

	/** How many of the night insects are out at {@code time}, 0 to 1: coming in through dusk, gone by dawn. */
	static float insectActivity(long time) {
		if (time >= 12000 && time < 13500) return (time - 12000) / 1500f;
		if (time >= 13500 && time < 22000) return 1;
		if (time >= 22000 && time < 23500) return (23500 - time) / 1500f;
		return 0;
	}

	/**
	 * Underground: now and then a stone working loose and settling somewhere off in the cave, and water dripping from
	 * the roof; Earshot gives them the cave's echo.
	 */
	private void caves(Minecraft minecraft, ClientLevel level) {
		if (ticks % 20 != 0 || minecraft.player == null) return;
		BlockPos ear = BlockPos.containing(minecraft.player.getEyePosition());
		boolean underground = level.getBrightness(net.minecraft.world.level.LightLayer.SKY, ear) == 0
				&& ear.getY() < level.getHeight(Heightmap.Types.WORLD_SURFACE, ear.getX(), ear.getZ()) - 8;
		if (!underground) return;
		BlockPos air = caveAir(level, ear);
		if (air == null) return;
		if (FoleyClient.wants(FoleyConfig::strikes) && random.nextFloat() < 0.04f) {
			call(minecraft, "settle_0", air, 0.5f);
		} else if (FoleyClient.wants(FoleyConfig::drips) && random.nextFloat() < 0.15f) {
			dripAt(minecraft, air);
		}
	}

	/** A spot on the floor of the cave 6 to 20 blocks off, or null. */
	private @Nullable BlockPos caveAir(ClientLevel level, BlockPos ear) {
		for (int tries = 0; tries < 6; tries++) {
			BlockPos at = ear.offset(random.nextInt(41) - 20, random.nextInt(9) - 4, random.nextInt(41) - 20);
			if (at.distSqr(ear) < 36 || !level.getBlockState(at).isAir()) continue;
			for (int down = 0; down < 8 && level.getBlockState(at.below()).isAir(); down++) at = at.below();
			if (!level.getBlockState(at.below()).isAir()) return at;
		}
		return null;
	}

	/** A bird that lives here, more or less likely by what the biome is like. */
	private int pickBird(Holder<Biome> biome, float celsius) {
		int[] weights; // crow, dove, robin, cardinal, chickadee, gull, loon (not by day), jay, warbler
		if (biome.is(BiomeTags.IS_OCEAN) || biome.is(BiomeTags.IS_BEACH)) {
			weights = new int[] {25, 15, 0, 0, 0, 60, 0, 0, 0};
		} else if (biome.is(BiomeTags.IS_TAIGA) || celsius < 6) {
			weights = new int[] {15, 0, 20, 0, 45, 0, 0, 10, 10};
		} else if (biome.is(BiomeTags.IS_FOREST)) {
			// Mostly songbirds: a crow or a jay now and then.
			weights = new int[] {4, 12, 28, 18, 18, 0, 0, 5, 15};
		} else if (biome.is(BiomeTags.IS_JUNGLE) || biome.is(Biomes.SWAMP) || biome.is(Biomes.MANGROVE_SWAMP)) {
			weights = new int[] {15, 40, 0, 30, 0, 0, 0, 0, 15};
		} else {
			weights = new int[] {10, 32, 25, 13, 5, 0, 0, 3, 12};
		}
		int roll = random.nextInt(100);
		for (int i = 0; i < weights.length; i++) {
			roll -= weights[i];
			if (roll < 0) return i;
		}
		return CROW;
	}

	private void call(Minecraft minecraft, String name, BlockPos at, float volume) {
		if (Sounds.play(minecraft, name, SoundSource.AMBIENT, at.getX() + 0.5, at.getY() + 0.2, at.getZ() + 0.5, volume,
				1f)) {
			calls++;
		}
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
