package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.natives.NativeBridge;
import net.minecraft.client.Minecraft;
import net.minecraft.resources.Identifier;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.vehicle.minecart.AbstractMinecart;
import net.minecraft.world.phys.Vec3;

/**
 * A small manager for minecart rush (M21): each minecart within 16 blocks gets a
 * low noise-rumble one-shot, gain proportional to its speed. Capped at
 * {@value #MAX_BEDS} beds at once.
 */
public final class MinecartBed {
	private static final int RADIUS = 16;
	/** How many minecart rush beds can play at once. Within the 8-stream budget. */
	private static final int MAX_BEDS = 2;
	/** Minimum speed to play a rush. */
	private static final float MIN_SPEED = 0.5f;

	private final NativeBridge bridge;
	private final java.util.Set<Integer> seen = new java.util.HashSet<>();

	public MinecartBed(NativeBridge bridge) {
		this.bridge = bridge;
	}

	/** Scans for nearby minecarts and plays a low rumble one-shot per moving cart. */
	public void tick(Minecraft minecraft) {
		if (minecraft.level == null || minecraft.player == null) return;
		Vec3 ear = minecraft.player.getEyePosition();
		java.util.Set<Integer> now = new java.util.HashSet<>();
		int allowed = 0;
		for (Entity e : minecraft.level.entitiesForRendering()) {
			if (!(e instanceof AbstractMinecart cart)) continue;
			Vec3 pos = e.position();
			if (ear.distanceToSqr(pos) > RADIUS * RADIUS) continue;
			now.add(e.getId());
			float speed = (float) e.getDeltaMovement().horizontalDistance();
			if (speed < MIN_SPEED) continue;
			allowed++;
			if (allowed > MAX_BEDS) continue;
			if (!FoleyClient.oneShot()) continue;
			// A short low rumble: noise primitive at 80 Hz, 0.5 s.
			short[] pcm = bridge.renderPrimitive(NativeBridge.PRIMITIVE_NOISE,
					new float[] {80.0f, 1.0f, 0.5f, Math.min(1.0f, speed * 0.3f)},
					e.getId() * 31L + minecraft.level.getGameTime());
			if (pcm == null) continue;
			minecraft.getSoundManager().play(new BufferSound(MINECART_RUSH,
					(0.15f + 0.15f * speed) * FoleyClient.gain(), pos.x, pos.y, pos.z, pcm));
		}
		seen.retainAll(now);
	}

	private static final Identifier MINECART_RUSH = Identifier.fromNamespaceAndPath("foley", "minecart_rush");
}
