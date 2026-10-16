package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import java.util.HashMap;
import java.util.Map;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.Mob;
import net.minecraft.world.entity.animal.chicken.Chicken;
import net.minecraft.world.entity.animal.sheep.Sheep;
import net.minecraft.world.entity.monster.spider.Spider;
import net.minecraft.world.phys.Vec3;

/**
 * Sheep, spiders and chickens brushing through plants as they wander (M22). The same {@link Brush#plant} classifier
 * the player uses picks grass, leaves or dry stalks at the mob's feet; the {@code brush_N} event fires for each stride
 * of movement.
 */
public final class MobBrushes {
	/** The mobs M22 adds to the brush scan. */
	public static final Class<?>[] MOB_KINDS = {Sheep.class, Spider.class, Chicken.class};
	/** How far the scan looks, blocks. */
	private static final int REACH = 16;

	private final Map<Integer, Vec3> last = new HashMap<>();
	private final java.util.Random random = new java.util.Random();

	public void tick(Minecraft minecraft) {
		if (minecraft.level == null || minecraft.player == null) return;
		if (!FoleyClient.wants(FoleyConfig::mobBrushes)) {
			last.clear();
			return;
		}
		Vec3 ear = minecraft.player.getEyePosition();
		for (Entity e : minecraft.level.entitiesForRendering()) {
			if (!(e instanceof Mob mob)) continue;
			if (!isBrushable(mob)) continue;
			if (e.distanceToSqr(ear) > REACH * REACH) continue;
			Vec3 pos = e.position();
			Vec3 before = last.put(e.getId(), pos);
			if (before == null) continue;
			double moved = Math.hypot(pos.x - before.x, pos.z - before.z);
			if (moved < 0.01 || moved > 2) continue;
			int plant = Brush.plant(minecraft.level.getBlockState(BlockPos.containing(pos)));
			if (plant < 0) continue;
			float volume = Math.clamp((float) (0.08 + 0.04 * moved * 20), 0.08f, 0.3f);
			float pitch = 0.95f + 0.1f * random.nextFloat();
			Sounds.play(minecraft, "brush_" + plant, SoundSource.NEUTRAL, pos.x, pos.y + 0.1, pos.z,
					volume * FoleyClient.gain(), pitch);
		}
	}

	private static boolean isBrushable(Mob mob) {
		for (Class<?> kind : MOB_KINDS) {
			if (kind.isInstance(mob)) return true;
		}
		return false;
	}
}