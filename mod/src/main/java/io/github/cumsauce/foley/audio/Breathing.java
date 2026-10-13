package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import java.util.HashMap;
import java.util.Map;
import java.util.UUID;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.EntityBoundSoundInstance;
import net.minecraft.sounds.SoundSource;
import net.minecraft.util.RandomSource;
import net.minecraft.world.entity.monster.zombie.Drowned;
import net.minecraft.world.entity.monster.zombie.Husk;
import net.minecraft.world.entity.monster.zombie.Zombie;
import net.minecraft.world.entity.monster.zombie.ZombieVillager;

/**
 * Zombies breathing (M7), when Foley voices them: within {@value #REACH} blocks, each one's breath follows it, louder,
 * heavier and faster the closer it is: steady breaths every 3 s or so at the edge, hard and ragged every second within
 * {@value #HEAVY} blocks. One that's after the listener (arms up) is worse: it seems nearer, breathes heavy from
 * further, faster, a little lower; within {@value #SNARL} blocks it often snarls instead of breathing. The four nearest
 * breathe; a drowned under water doesn't.
 */
public final class Breathing {
	static final double REACH = 10;
	static final double HEAVY = 4.5;
	static final double SNARL = 3.5;
	private static final int MOST = 4;

	private final Map<UUID, Integer> next = new HashMap<>();
	private final RandomSource random = RandomSource.create();
	private int ticks;
	private int breaths;

	/** Breaths so far. For the client gametest. */
	public int breaths() {
		return breaths;
	}

	public void tick(Minecraft minecraft) {
		ticks++;
		if (minecraft.level == null || minecraft.player == null || !Voices.voiced("zombie")) {
			next.clear();
			return;
		}
		var eye = minecraft.player.getEyePosition();
		var near = minecraft.level.getEntitiesOfClass(Zombie.class, minecraft.player.getBoundingBox().inflate(REACH),
				z -> z.isAlive() && !z.isSilent() && z.distanceTo(minecraft.player) <= REACH);
		near.sort((a, b) -> Double.compare(a.distanceToSqr(eye), b.distanceToSqr(eye)));
		if (near.size() > MOST) near = near.subList(0, MOST);
		var keep = new HashMap<UUID, Integer>();
		for (Zombie zombie : near) {
			if (zombie instanceof Drowned && zombie.isUnderWater()) continue;
			boolean hunting = zombie.isAggressive();
			// Hunting, it seems nearer than it is.
			double distance = Math.sqrt(zombie.distanceToSqr(eye)) / (hunting ? 1.4 : 1);
			double closeness = Math.clamp(1 - distance / REACH, 0, 1);
			int due = next.getOrDefault(zombie.getUUID(), ticks + random.nextInt(40));
			if (ticks >= due) {
				boolean heavy = distance < HEAVY;
				boolean snarl = distance < SNARL && random.nextFloat() < (hunting ? 0.45f : 0.2f);
				String mob = zombie instanceof Husk ? "husk"
						: zombie instanceof Drowned ? "drowned"
						: zombie instanceof ZombieVillager ? "zombie_villager" : "zombie";
				float volume = (float) Math.min(1, 0.25 + 0.7 * closeness + (snarl ? 0.15 : 0)) * FoleyClient.gain();
				float pitch = (zombie.isBaby() ? 1.4f : 0.95f + 0.1f * random.nextFloat()) * (hunting ? 0.93f : 1f);
				String call = snarl ? "growl" : heavy ? "breath_heavy" : "breath";
				minecraft.getSoundManager().play(new EntityBoundSoundInstance(Voices.event(mob, call),
						SoundSource.HOSTILE, volume, pitch, zombie, random.nextLong()));
				breaths++;
				// A breath takes 1 to 3 s; the next follows sooner the closer it is.
				due = ticks + (int) (60 - 38 * closeness) + random.nextInt(15) + (snarl ? 20 : 0);
			}
			keep.put(zombie.getUUID(), due);
		}
		next.clear();
		next.putAll(keep);
	}
}
