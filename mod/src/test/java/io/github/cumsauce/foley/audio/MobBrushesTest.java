package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertTrue;

import net.minecraft.world.entity.animal.chicken.Chicken;
import net.minecraft.world.entity.animal.sheep.Sheep;
import net.minecraft.world.entity.monster.spider.Spider;
import org.junit.jupiter.api.Test;

/** Mob brushes (M22): the kinds of mob the scan looks for. */
class MobBrushesTest {
	@Test
	void theMobsAreSheepSpidersAndChickens() {
		// The M22 spec names sheep, spiders and chickens — peaceful mobs whose
		// legs and bodies push through grass, leaves and feathers.
		Class<?>[] kinds = MobBrushes.MOB_KINDS;
		assertTrue(java.util.Arrays.asList(kinds).contains(Sheep.class), "sheep");
		assertTrue(java.util.Arrays.asList(kinds).contains(Spider.class), "spider");
		assertTrue(java.util.Arrays.asList(kinds).contains(Chicken.class), "chicken");
		assertTrue(kinds.length == 3, "no extras: " + kinds.length);
	}
}