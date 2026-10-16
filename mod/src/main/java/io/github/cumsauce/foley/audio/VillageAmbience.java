package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import io.github.cumsauce.foley.natives.NativeBridge;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.resources.Identifier;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.animal.Animal;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;

/**
 * Village ambience (M20): the world's villages get to be audible. The bell rings (one of the
 * most recognisable vanilla sounds; Foley doesn't replace it but layers its own ring under it),
 * the murmur (low-passed voices and animal calls) plays at the bell block at a Poisson rate,
 * the smith's anvil rings (an anvil adjacent to a blast furnace), and scattered animals around
 * coops and fences make their sounds. Runs every {@link #SAMPLE_EVERY} ticks.
 *
 * <p>Bell rings scale with the weather: the bell rings more often in a storm. The murmur is one
 * shared low-volume voice call per bell every few seconds. The smith is per anvil. Each block
 * position is rate-limited so a high-velocity scan doesn't fire dozens of sounds per second.
 */
public final class VillageAmbience {
	/** Per-tick scan cadence (10 ticks = 0.5 s). */
	private static final int SAMPLE_EVERY = 10;
	/** How far the scan looks for bells, smiths and animals, blocks. */
	private static final int RADIUS = 64;
	/** The scan step (blocks), to keep the scan budget small. */
	private static final int SCAN_STEP = 4;

	private static final Identifier BELL_EVENT = Identifier.fromNamespaceAndPath("foley", "village_bell");
	private static final Identifier MURMUR_EVENT = Identifier.fromNamespaceAndPath("foley", "village_murmur");
	private static final Identifier SMITH_EVENT = Identifier.fromNamespaceAndPath("foley", "smith_anvil");

	private final NativeBridge bridge;
	private final VillageBed beds;
	private long tickCount;
	private int played;

	public VillageAmbience(NativeBridge bridge) {
		this.bridge = bridge;
		this.beds = new VillageBed(bridge);
	}

	/** Total M20 events Foley has played this session. */
	public int played() {
		return played;
	}

	/** The shared per-village bed manager. */
	public VillageBed beds() {
		return beds;
	}

	/** Ticks the M20 systems. Client thread. */
	public void tick(Minecraft minecraft) {
		if (minecraft.level == null || minecraft.player == null) return;
		if (!FoleyClient.wants(FoleyConfig::village)) return;
		tickCount++;
		if (tickCount % SAMPLE_EVERY != 0) return;
		long now = tickCount;
		// Bell rings more often in a storm: shrink the gap from 600 to 200 ticks.
		float rain = minecraft.level.getRainLevel(1f);
		float thunder = minecraft.level.getThunderLevel(1f);
		long bellGapTicks = (long) (600 - 400 * Math.max(rain, thunder));
		long murmurGapTicks = 6 + (long) (4 * Math.random()); // 0.3 to 0.5 s per scan
		long animalGapTicks = 100 + (long) (60 * Math.random()); // 5 to 8 s per scan
		scanBells(minecraft, now, bellGapTicks, murmurGapTicks);
		scanSmiths(minecraft, now);
		scanAnimals(minecraft, now, animalGapTicks);
		beds.prune();
	}

	private void scanBells(Minecraft minecraft, long now, long bellGap, long murmurGap) {
		int px = (int) Math.floor(minecraft.player.getX());
		int pz = (int) Math.floor(minecraft.player.getZ());
		for (int dx = -RADIUS; dx <= RADIUS; dx += SCAN_STEP) {
			for (int dz = -RADIUS; dz <= RADIUS; dz += SCAN_STEP) {
				int x = px + dx;
				int z = pz + dz;
				int top = minecraft.level.getHeight(net.minecraft.world.level.levelgen.Heightmap.Types.MOTION_BLOCKING, x, z);
				for (int y = minecraft.level.getMinY(); y <= top; y++) {
					BlockPos pos = new BlockPos(x, y, z);
					if (!minecraft.level.getBlockState(pos).is(Blocks.BELL)) continue;
					beds.see(pos);
					if (!beds.admitBell(minecraft, pos)) continue;
					// Bell ring (sparse; rate-limited per bell).
					if (beds.bellShouldRing(pos, now, bellGap)) {
						playBell(minecraft, pos);
					}
					// Murmur (a low-volume villager voice one-shot at the bell).
					if (beds.murmurShouldFire(pos, now, murmurGap)) {
						playMurmur(minecraft, pos);
					}
				}
			}
		}
	}

	private void playBell(Minecraft minecraft, BlockPos pos) {
		if (!FoleyClient.oneShot()) return;
		long seed = pos.asLong();
		short[] pcm = bridge.renderOneShot(NativeBridge.SHOT_BELL, 8f, seed);
		if (pcm == null) return;
		// 48 kHz mono 16-bit: 0.1 s of audio at 0.7 gain is loud but not loud enough to drown footsteps.
		beds.playBuffer(BELL_EVENT, pcm, pos.getX() + 0.5, pos.getY() + 0.5, pos.getZ() + 0.5,
				0.7f * FoleyClient.gain());
		played++;
	}

	private void playMurmur(Minecraft minecraft, BlockPos pos) {
		if (!FoleyClient.oneShot()) return;
		long seed = pos.asLong() ^ 0x9E3779B97F4A7C15L;
		// The villager's ambient call ("mm-hm"), rendered small for the murmur. The mob
		// index follows VoiceTable.MOBS, which mirrors the native Mob enum's order.
		int villagerMob = VoiceTable.MOBS.indexOf("villager");
		try {
			short[] pcm = bridge.renderVoice(villagerMob, 0, seed);
			if (pcm == null) return;
			beds.playBuffer(MURMUR_EVENT, pcm, pos.getX() + 0.5, pos.getY() + 0.5, pos.getZ() + 0.5,
					0.15f * FoleyClient.gain());
			played++;
		} catch (Throwable t) {
			// Voice rendering may fail for an unrecognised mob index; skip silently.
		}
	}

	private void scanSmiths(Minecraft minecraft, long now) {
		// A smith is an anvil next to a blast furnace (4 cardinal neighbours).
		int px = (int) Math.floor(minecraft.player.getX());
		int pz = (int) Math.floor(minecraft.player.getZ());
		for (int dx = -RADIUS; dx <= RADIUS; dx += SCAN_STEP) {
			for (int dz = -RADIUS; dz <= RADIUS; dz += SCAN_STEP) {
				int x = px + dx;
				int z = pz + dz;
				int top = minecraft.level.getHeight(net.minecraft.world.level.levelgen.Heightmap.Types.MOTION_BLOCKING, x, z);
				for (int y = minecraft.level.getMinY(); y <= top; y++) {
					BlockPos pos = new BlockPos(x, y, z);
					Block b = minecraft.level.getBlockState(pos).getBlock();
					if (!isAnvil(b)) continue;
					boolean nextToBlast = false;
					for (BlockPos n : new BlockPos[] {pos.north(), pos.south(), pos.east(), pos.west()}) {
						if (minecraft.level.getBlockState(n).is(Blocks.BLAST_FURNACE)) {
							nextToBlast = true;
							break;
						}
					}
					if (!nextToBlast) continue;
					if (!FoleyClient.oneShot()) return;
					long seed = pos.asLong();
					short[] pcm = bridge.renderContact(NativeBridge.CONTACT_TAP, nativeMetal(), 2.0f, seed);
					if (pcm == null) continue;
					// The smith fires rapidly — rate limit per anvil.
					beds.see(pos); // record smith position; same prune path
					if (!beds.animalShouldFire(pos, now, 10)) continue;
					beds.playBuffer(SMITH_EVENT, pcm, pos.getX() + 0.5, pos.getY() + 0.5, pos.getZ() + 0.5,
							0.3f * FoleyClient.gain());
					played++;
				}
			}
		}
	}

	private void scanAnimals(Minecraft minecraft, long now, long gap) {
		// Chickens by hay blocks (coops), cows/animals by fences. Scan 8 cubes around player.
		int px = (int) Math.floor(minecraft.player.getX());
		int pz = (int) Math.floor(minecraft.player.getZ());
		for (Entity e : minecraft.level.entitiesForRendering()) {
			if (!(e instanceof Animal animal)) continue;
			BlockPos here = e.blockPosition();
			if (Math.abs(here.getX() - px) > RADIUS || Math.abs(here.getZ() - pz) > RADIUS) continue;
			boolean nearHay = false;
			boolean nearFence = false;
			for (BlockPos n : new BlockPos[] {here.north(), here.south(), here.east(), here.west(),
					here.above(), here.below()}) {
				Block b = minecraft.level.getBlockState(n).getBlock();
				if (b == Blocks.HAY_BLOCK) nearHay = true;
				if (isFence(b)) nearFence = true;
			}
			if (!nearHay && !nearFence) continue;
			if (!FoleyClient.oneShot()) continue;
			beds.see(here);
			if (!beds.animalShouldFire(here, now, gap)) continue;
			// A low-volume bird/animal one-shot at the structure position.
			short[] pcm = bridge.renderOneShot(NativeBridge.SHOT_BIRDSONG, 0f, e.getId() + (long) here.hashCode());
			if (pcm == null) continue;
			beds.playBuffer(MURMUR_EVENT, pcm, here.getX() + 0.5, here.getY(), here.getZ() + 0.5,
					0.1f * FoleyClient.gain());
			played++;
		}
	}

	private static boolean isAnvil(Block b) {
		// The anvil block has three damage states; the IDs are ANVIL, CHIPPED_ANVIL, DAMAGED_ANVIL.
		return b == Blocks.ANVIL || b == Blocks.CHIPPED_ANVIL || b == Blocks.DAMAGED_ANVIL;
	}

	private static boolean isFence(Block b) {
		// An over-broad check; the per-tick animal scan is cheap enough that a few false
		// positives are fine.
		String name = net.minecraft.core.registries.BuiltInRegistries.BLOCK.getKey(b).getPath();
		return name.endsWith("_fence") || name.endsWith("_fence_gate") || name.contains("wall");
	}

	/** {@code MATERIAL_METAL = 2} in {@code FOLEY_MATERIALS}. */
	private static int nativeMetal() {
		return 2;
	}
}
