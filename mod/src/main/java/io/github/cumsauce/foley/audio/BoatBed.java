package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.natives.NativeBridge;
import net.minecraft.client.Minecraft;
import net.minecraft.resources.Identifier;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.vehicle.boat.AbstractBoat;
import net.minecraft.world.phys.Vec3;

/**
 * A small manager for boat lapping (M21): each boat within 16 blocks gets bubble
 * one-shots at a Poisson rate, faster the faster the boat moves. Capped at
 * {@value #MAX_BOATS} boats at once.
 */
public final class BoatBed {
	private static final int RADIUS = 16;
	/** How many boat beds can play at once. Within the 8-stream budget. */
	private static final int MAX_BOATS = 2;

	private final NativeBridge bridge;
	/** The boat ids currently tracked. */
	private final java.util.Set<Integer> seen = new java.util.HashSet<>();

	public BoatBed(NativeBridge bridge) {
		this.bridge = bridge;
	}

	/** Scans for nearby boats and plays bubble one-shots at each. */
	public void tick(Minecraft minecraft) {
		if (minecraft.level == null || minecraft.player == null) return;
		Vec3 ear = minecraft.player.getEyePosition();
		java.util.Set<Integer> now = new java.util.HashSet<>();
		int tracked = 0;
		for (Entity e : minecraft.level.entitiesForRendering()) {
			if (!(e instanceof AbstractBoat boat)) continue;
			Vec3 pos = e.position();
			if (ear.distanceToSqr(pos) > RADIUS * RADIUS) continue;
			int id = e.getId();
			now.add(id);
			if (seen.add(id)) tracked++;
		}
		seen.retainAll(now);
		// At most MAX_BOATS boats can lap; the rest are silent (cap is enforced by skip).
		int allowed = 0;
		for (Entity e : minecraft.level.entitiesForRendering()) {
			if (!(e instanceof AbstractBoat boat)) continue;
			Vec3 pos = e.position();
			if (ear.distanceToSqr(pos) > RADIUS * RADIUS) continue;
			allowed++;
			if (allowed > MAX_BOATS) continue;
			if (!FoleyClient.oneShot()) continue;
			// A bubble primitive at 5 mm radius, gain lower if the boat is still.
			float speed = (float) e.getDeltaMovement().horizontalDistance();
			float rate = 1.0f + 4.0f * speed;
			if (minecraft.level.getRandom().nextFloat() > rate / 20.0f) continue;
			short[] pcm = bridge.renderPrimitive(NativeBridge.PRIMITIVE_BUBBLE, new float[] {5.0f}, e.getId() * 31L + minecraft.level.getGameTime());
			if (pcm == null) continue;
			minecraft.getSoundManager().play(new BufferSound(BOAT_LAP,
					(0.15f + 0.1f * speed) * FoleyClient.gain(), pos.x, pos.y, pos.z, pcm));
		}
	}

	private static final Identifier BOAT_LAP = Identifier.fromNamespaceAndPath("foley", "boat_lap");
}
