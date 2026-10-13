package io.github.cumsauce.foley.audio;

import java.util.UUID;
import net.minecraft.client.Minecraft;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.Mob;
import net.minecraft.world.entity.monster.cubemob.Slime;
import net.minecraft.world.phys.AABB;
import org.jspecify.annotations.Nullable;

/**
 * Each mob's own voice. The game varies a mob's pitch at random with every call, so no cow sounds like itself twice;
 * real animals of a kind differ in size and so in voice, and each keeps its own. Here a mob's voice pitch is fixed by
 * its UUID, which the world saves and sends to the client, so it is the same every time it calls and every time the
 * world is loaded, with only a slight wobble call to call. Pitch moves the voice's fundamental and its formants
 * together, as a bigger or smaller body would.
 *
 * <p>Within that voice, it still varies as a living one does: each mob has a few of a call's renditions it mostly
 * uses (its habits: two of four, three of twelve), and now and then another; and it calls a little higher when roused
 * (just hurt, angry, or on the run), as animals do.
 */
public final class Individuals {
	/** How far one mob's voice pitch may be from the middle of its kind's: about a semitone each way. */
	private static final float SPREAD = 0.07f;
	/** How much a mob's pitch still wavers call to call. */
	private static final float WOBBLE = 0.02f;
	/** The game's pitch for a baby is about 1.5, an adult's about 1; between them, babies. */
	private static final float BABY = 1.25f;
	/** How often a mob calls in one of its habitual ways. */
	private static final float HABIT = 0.7f;

	/**
	 * A call as this mob makes it: at {@code pitch}, by the mob {@code id}, with {@code roll} and {@code any} (uniform in
	 * [0, 1)) choosing the rendition once their number is known.
	 */
	public record Voice(float pitch, UUID id, float roll, float any) {
		/** Which of {@code count} renditions this call is. */
		public int rendition(int count) {
			return Individuals.rendition(id, roll, Math.min(count - 1, (int) (any * count)), count);
		}
	}

	private Individuals() {}

	/**
	 * The pitch for a call of {@code mob} (Foley's name, which is the entity type's) heard at ({@code x}, {@code y},
	 * {@code z}) with the game's {@code pitch}: the caller's own when it can be found, else the game's. Client thread.
	 */
	public static @Nullable Voice of(String mob, double x, double y, double z, float pitch) {
		Entity caller = find(mob, x, y, z);
		// Slimes' pitch says how big they are; leave it.
		if (!(caller instanceof Mob creature) || caller instanceof Slime) return null;
		var random = caller.getRandom();
		float base = pitch > BABY ? 1.5f : 1f;
		float wobble = 1 + WOBBLE * (2 * random.nextFloat() - 1);
		// Roused: just hurt, angry, or on the run.
		float roused = 1;
		if (creature.hurtTime > 0) roused += 0.04f;
		else if (creature.isAggressive()) roused += 0.03f;
		if (creature.getDeltaMovement().horizontalDistanceSqr() > 0.02) roused += 0.015f;
		return new Voice(base * voice(caller.getUUID()) * wobble * roused, caller.getUUID(), random.nextFloat(),
				random.nextFloat());
	}

	/**
	 * Which of {@code count} renditions a mob uses this time: with probability {@value #HABIT} one of its habitual
	 * ones (a quarter of them, at least one; which, from its UUID), else {@code any}. {@code roll} is uniform in
	 * [0, 1).
	 */
	static int rendition(UUID id, float roll, int any, int count) {
		if (count <= 1) return 0;
		if (roll >= HABIT) return any;
		int habits = Math.max(1, count / 4);
		long bits = (id.getLeastSignificantBits() ^ Long.rotateLeft(id.getMostSignificantBits(), 29))
				* 0xC2B2AE3D27D4EB4FL;
		// Habits spread evenly through the renditions from a start the UUID picks.
		int start = (int) Long.remainderUnsigned(bits >>> 8, count);
		int which = Math.min(habits - 1, (int) (roll / HABIT * habits));
		return (start + which * (count / habits)) % count;
	}

	/**
	 * A mob's own voice pitch from its UUID: 1 ± {@value #SPREAD}, more often near the middle (the sum of two even
	 * draws, as sizes in a population cluster round the typical).
	 */
	static float voice(UUID id) {
		long bits = id.getMostSignificantBits() ^ Long.rotateLeft(id.getLeastSignificantBits(), 17);
		bits *= 0x9E3779B97F4A7C15L;
		float a = (bits >>> 40) / (float) (1L << 24);
		float b = ((bits >>> 16) & 0xFFFFFF) / (float) (1L << 24);
		return 1 + SPREAD * (a + b - 1);
	}

	/** The mob of type {@code mob} nearest ({@code x}, {@code y}, {@code z}), within a block or two, or null. */
	private static @Nullable Entity find(String mob, double x, double y, double z) {
		var level = Minecraft.getInstance().level;
		if (level == null) return null;
		Entity best = null;
		double bestDistance = Double.MAX_VALUE;
		for (Entity entity : level.getEntities((Entity) null, new AABB(x - 2, y - 2, z - 2, x + 2, y + 2, z + 2),
				entity -> BuiltInRegistries.ENTITY_TYPE.getKey(entity.getType()).getPath().equals(mob))) {
			double distance = entity.distanceToSqr(x, y, z);
			if (distance < bestDistance) {
				best = entity;
				bestDistance = distance;
			}
		}
		return best;
	}
}
