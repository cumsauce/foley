package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.EntityBoundSoundInstance;
import net.minecraft.sounds.SoundSource;
import net.minecraft.util.RandomSource;
import net.minecraft.world.entity.Mob;
import net.minecraft.world.entity.animal.cow.Cow;
import net.minecraft.world.entity.animal.sheep.Sheep;
import net.minecraft.world.phys.Vec3;

/**
 * Cows and sheep breathing when Foley voices them (M22): within {@link #REACH} blocks, one breath every 3 s, very
 * quietly — a peaceful mob in a barn at night. A baby's breath is a touch higher in pitch; a mob at the edge of
 * {@link #REACH} is barely audible.
 */
public final class PeacefulBreathing {
	/** How far the scan looks, blocks. The same as {@link Breathing#REACH}. */
	public static final double REACH = 10;
	/** The mobs M22 adds to the peaceful-breathing scan. */
	public static final Class<?>[] KINDS = {Cow.class, Sheep.class};

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
		if (minecraft.level == null || minecraft.player == null) {
			next.clear();
			return;
		}
		if (!FoleyClient.wants(FoleyConfig::peacefulBreathing)) {
			next.clear();
			return;
		}
		// The mob needs to be voiced for its breath to play.
		if (!Voices.voiced("cow") && !Voices.voiced("sheep")) {
			next.clear();
			return;
		}
		Vec3 eye = minecraft.player.getEyePosition();
		List<Mob> near = new ArrayList<>();
		for (Class<?> kind : KINDS) {
			@SuppressWarnings("unchecked")
			Class<Mob> mobKind = (Class<Mob>) kind;
			minecraft.level.getEntitiesOfClass(mobKind, minecraft.player.getBoundingBox().inflate(REACH),
					m -> m.isAlive() && !m.isSilent() && m.distanceTo(minecraft.player) <= REACH)
					.forEach(near::add);
		}
		near.sort((a, b) -> Double.compare(a.distanceToSqr(eye), b.distanceToSqr(eye)));
		var keep = new HashMap<UUID, Integer>();
		for (Mob mob : near) {
			int due = next.getOrDefault(mob.getUUID(), ticks + random.nextInt(40));
			if (ticks >= due) {
				String which = mob instanceof Sheep ? "sheep" : "cow";
				if (!Voices.voiced(which)) continue;
				double distance = Math.sqrt(mob.distanceToSqr(eye));
				float volume = (float) Math.min(0.25, 0.05 + 0.12 * (1 - distance / REACH)) * FoleyClient.gain();
				float pitch = mob.isBaby() ? 1.3f : 0.95f + 0.06f * random.nextFloat();
				minecraft.getSoundManager().play(new EntityBoundSoundInstance(Voices.event(which, "breath"),
						SoundSource.NEUTRAL, volume, pitch, mob, random.nextLong()));
				breaths++;
				due = ticks + 60 + random.nextInt(15);
			}
			keep.put(mob.getUUID(), due);
		}
		next.clear();
		next.putAll(keep);
	}
}