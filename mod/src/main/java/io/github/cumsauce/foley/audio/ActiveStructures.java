package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import io.github.cumsauce.foley.natives.NativeBridge;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.core.BlockPos;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.vehicle.boat.AbstractBoat;
import net.minecraft.world.entity.vehicle.minecart.AbstractMinecart;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.entity.BlockEntity;
import net.minecraft.world.level.block.entity.SculkSensorBlockEntity;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.Vec3;
import org.jspecify.annotations.Nullable;

/**
 * Active structures (M21): minecarts, doors, chests, redstone, boats, sculk. The
 * per-10-tick scan finds the ones within {@value #RADIUS} blocks of the player; the
 * per-event hooks ({@link #onDoorSkipped}, {@link #onChestSkipped}) are called by
 * {@link Replacements} when the game plays a vanilla event Foley is taking over.
 * Within the scan, minecart wheels, redstone state changes and sculk sensor triggers
 * become one-shots; boats and minecarts have a per-entity bed.
 */
public final class ActiveStructures {
	/** Per-tick scan cadence (10 ticks = 0.5 s). */
	private static final int SAMPLE_EVERY = 10;
	/** How far the scan looks for blocks and entities, blocks. */
	private static final int RADIUS = 16;
	/** How far an event hook will replace a game sound, blocks. */
	private static final int HOOK_RADIUS = 16;
	/** The 8 cubes around the player scanned in redstone/sculk loops. */
	private static final int SCAN_STEP = 4;

	private final NativeBridge bridge;
	private final BoatBed boats;
	private final MinecartBed rush;
	/** Last minecart position per entity id, for the wheel-impact distance gate. */
	private final Map<Integer, Vec3> lastCart = new HashMap<>();
	/** Last redstone component state hash per BlockPos, for delta detection. */
	private final Map<BlockPos, Integer> lastRedstone = new HashMap<>();
	/** Last sculk sensor vibration frequency per BlockPos, for trigger detection. */
	private final Map<BlockPos, Integer> lastSensor = new HashMap<>();
	/** Total M21 events Foley has played. For the gametest. */
	private int played;
	private int ticks;

	public ActiveStructures(NativeBridge bridge) {
		this.bridge = bridge;
		this.boats = new BoatBed(bridge);
		this.rush = new MinecartBed(bridge);
	}

	/** Total M21 events Foley has played this session. */
	public int played() {
		return played;
	}

	/** Static accessor for the active-structures counter. For the gametest. */
	public static int playedTotal(Minecraft minecraft) {
		var active = FoleyClient.activeStructures();
		return active == null ? 0 : active.played();
	}

	/** The boat lapping manager. */
	public BoatBed boats() {
		return boats;
	}

	/** The minecart rush manager. */
	public MinecartBed rush() {
		return rush;
	}

	/** Ticks the M21 systems. Client thread. */
	public void tick(Minecraft minecraft) {
		if (minecraft.level == null || minecraft.player == null) return;
		ticks++;
		if (ticks % SAMPLE_EVERY != 0) return;
		if (FoleyClient.wants(FoleyConfig::minecarts)) {
			scanMinecarts(minecraft);
			rush.tick(minecraft);
		}
		if (FoleyClient.wants(FoleyConfig::boats)) boats.tick(minecraft);
		if (FoleyClient.wants(FoleyConfig::redstone)) scanRedstone(minecraft);
		if (FoleyClient.wants(FoleyConfig::sculk)) scanSculk(minecraft);
	}

	/** Called by {@link Replacements} when a door open/close event is skipped. */
	public void onDoorSkipped(Minecraft minecraft, SoundInstance sound, boolean iron) {
		if (!within(minecraft, sound, HOOK_RADIUS)) return;
		if (!FoleyClient.oneShot()) return;
		int material = iron ? Materials.METAL : Materials.WOOD;
		String event = Layers.CONTACTS[NativeBridge.CONTACT_TAP] + "_" + Materials.NAMES[material];
		minecraft.getSoundManager().play(new SimpleSoundInstance(Sounds.event(event), SoundSource.BLOCKS,
				0.5f * FoleyClient.gain(), 1f, SoundInstance.createUnseededRandom(),
				sound.getX(), sound.getY(), sound.getZ()));
		played++;
	}

	/** Called by {@link Replacements} when a chest or shulker open/close is skipped. */
	public void onChestSkipped(Minecraft minecraft, SoundInstance sound, boolean shulker) {
		if (!within(minecraft, sound, HOOK_RADIUS)) return;
		if (!FoleyClient.oneShot()) return;
		if (shulker) {
			// A small puff: a bubble primitive at 4 mm radius. We render the buffer and play it.
			short[] pcm = bridge.renderPrimitive(NativeBridge.PRIMITIVE_BUBBLE, new float[] {4.0f}, 1L);
			playBuffer(SHULKER_OPEN, pcm, sound.getX(), sound.getY(), sound.getZ(), 0.4f * FoleyClient.gain());
		} else {
			String event = Layers.CONTACTS[NativeBridge.CONTACT_TAP] + "_" + Materials.NAMES[Materials.WOOD];
			minecraft.getSoundManager().play(new SimpleSoundInstance(Sounds.event(event), SoundSource.BLOCKS,
					0.4f * FoleyClient.gain(), 1f, SoundInstance.createUnseededRandom(),
					sound.getX(), sound.getY(), sound.getZ()));
		}
		played++;
	}

	private void scanMinecarts(Minecraft minecraft) {
		Vec3 ear = minecraft.player.getEyePosition();
		Set<Integer> seen = new HashSet<>();
		for (Entity e : minecraft.level.entitiesForRendering()) {
			if (!(e instanceof AbstractMinecart cart)) continue;
			Vec3 pos = e.position();
			if (ear.distanceToSqr(pos) > RADIUS * RADIUS) continue;
			int id = e.getId();
			seen.add(id);
			Vec3 last = lastCart.get(id);
			// The rail under the cart, for wood vs metal (powered rail) timbre.
			BlockState rail = minecraft.level.getBlockState(e.blockPosition());
			boolean powered = rail.is(Blocks.POWERED_RAIL);
			// A wheel impact every ~0.4 s of travel.
			if (last == null || last.distanceTo(pos) > 0.3) {
				int material = powered ? Materials.METAL : Materials.WOOD;
				float volume = powered ? 0.7f : 0.35f;
				String event = Layers.CONTACTS[NativeBridge.CONTACT_TAP] + "_" + Materials.NAMES[material];
				minecraft.getSoundManager().play(new SimpleSoundInstance(Sounds.event(event), SoundSource.BLOCKS,
						volume * FoleyClient.gain(), 1f, SoundInstance.createUnseededRandom(),
						pos.x, pos.y, pos.z));
				played++;
				lastCart.put(id, pos);
			}
		}
		lastCart.keySet().retainAll(seen);
	}

	private void scanRedstone(Minecraft minecraft) {
		Vec3 ear = minecraft.player.getEyePosition();
		int px = (int) Math.floor(ear.x);
		int pz = (int) Math.floor(ear.z);
		Set<BlockPos> seen = new HashSet<>();
		for (int dx = -RADIUS; dx <= RADIUS; dx += SCAN_STEP) {
			for (int dz = -RADIUS; dz <= RADIUS; dz += SCAN_STEP) {
				int x = px + dx;
				int z = pz + dz;
				int top = minecraft.level.getHeight(net.minecraft.world.level.levelgen.Heightmap.Types.MOTION_BLOCKING, x, z);
				for (int y = minecraft.level.getMinY(); y <= top; y++) {
					BlockPos pos = new BlockPos(x, y, z);
					BlockEntity be = minecraft.level.getBlockEntity(pos);
					if (be == null) continue;
					Block b = be.getBlockState().getBlock();
					if (b != Blocks.REPEATER && b != Blocks.COMPARATOR) continue;
					seen.add(pos);
					// Coarse state hash: power level + delay (for repeater) or mode (for comparator).
					BlockState s = be.getBlockState();
					int state = s.getValue(net.minecraft.world.level.block.RedstoneWireBlock.POWER) * 17;
					if (b == Blocks.COMPARATOR) {
						state += s.getValue(net.minecraft.world.level.block.ComparatorBlock.MODE).ordinal() * 7;
					}
					Integer prev = lastRedstone.get(pos);
					if (prev != null && prev != state) {
						String event = Layers.CONTACTS[NativeBridge.CONTACT_TAP] + "_" + Materials.NAMES[Materials.STONE];
						minecraft.getSoundManager().play(new SimpleSoundInstance(Sounds.event(event),
								SoundSource.BLOCKS, 0.18f * FoleyClient.gain(), 1f,
								SoundInstance.createUnseededRandom(),
								pos.getX() + 0.5, pos.getY() + 0.5, pos.getZ() + 0.5));
						played++;
					}
					lastRedstone.put(pos, state);
				}
			}
		}
		lastRedstone.keySet().retainAll(seen);
	}

	private void scanSculk(Minecraft minecraft) {
		Vec3 ear = minecraft.player.getEyePosition();
		int px = (int) Math.floor(ear.x);
		int pz = (int) Math.floor(ear.z);
		for (int dx = -RADIUS; dx <= RADIUS; dx += SCAN_STEP) {
			for (int dz = -RADIUS; dz <= RADIUS; dz += SCAN_STEP) {
				int x = px + dx;
				int z = pz + dz;
				int top = minecraft.level.getHeight(net.minecraft.world.level.levelgen.Heightmap.Types.MOTION_BLOCKING, x, z);
				for (int y = minecraft.level.getMinY(); y <= top; y++) {
					BlockPos pos = new BlockPos(x, y, z);
					BlockEntity be = minecraft.level.getBlockEntity(pos);
					if (!(be instanceof SculkSensorBlockEntity sensor)) continue;
					int last = sensor.getLastVibrationFrequency();
					Integer prev = lastSensor.get(pos);
					if (prev != null && prev != last) {
						triggerSculkRing(minecraft, pos);
						for (int ddx = -8; ddx <= 8; ddx += 2) {
							for (int ddz = -8; ddz <= 8; ddz += 2) {
								for (int ddy = -4; ddy <= 4; ddy += 2) {
									BlockPos at = pos.offset(ddx, ddy, ddz);
									if (minecraft.level.getBlockState(at).is(Blocks.SCULK)) {
										triggerSculkRing(minecraft, at);
									}
								}
							}
						}
					}
					lastSensor.put(pos, last);
				}
			}
		}
	}

	private void triggerSculkRing(Minecraft minecraft, BlockPos at) {
		if (minecraft.player == null) return;
		if (minecraft.player.distanceToSqr(at.getX() + 0.5, at.getY() + 0.5, at.getZ() + 0.5) > 24 * 24) return;
		if (!FoleyClient.oneShot()) return;
		String event = "sculk_ring";
		minecraft.getSoundManager().play(new SimpleSoundInstance(Sounds.event(event), SoundSource.BLOCKS,
				0.3f * FoleyClient.gain(), 1f, SoundInstance.createUnseededRandom(),
				at.getX() + 0.5, at.getY() + 0.5, at.getZ() + 0.5));
		played++;
	}

	private static final Identifier SHULKER_OPEN = Identifier.fromNamespaceAndPath("foley", "shulker_open");

	private static boolean within(Minecraft minecraft, SoundInstance sound, int radius) {
		if (minecraft.level == null || minecraft.player == null) return false;
		return minecraft.player.distanceToSqr(sound.getX(), sound.getY(), sound.getZ()) <= (double) radius * radius;
	}

	/** Plays a pre-rendered PCM buffer as a one-shot, via the game's sound system. */
	private void playBuffer(Identifier event, short @Nullable [] pcm, double x, double y, double z, float volume) {
		if (pcm == null) return;
		Minecraft minecraft = Minecraft.getInstance();
		var sound = new BufferSound(event, volume, x, y, z, pcm);
		minecraft.getSoundManager().play(sound);
	}
}
