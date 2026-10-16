package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

class BiomeSceneTest {
	@Test
	void paramsMatchTheNativeOrder() {
		var s = new BiomeScene(BiomeScene.MANGROVE, 0.6f, 80f, 0.7f);
		var p = s.params();
		assertEquals(BiomeScene.MANGROVE, (int) p[0]);
		assertEquals(0.6f, p[1], 1e-6f);
		assertEquals(80f, p[2], 1e-6f);
		assertEquals(0.7f, p[3], 1e-6f);
	}

	@Test
	void silentIsAllZero() {
		var p = BiomeScene.SILENT.params();
		assertArrayEquals(new float[] {0f, 0f, 0f, 0f}, p, 1e-6f);
	}

	@Test
	void biomeKindConstantsAreSequential() {
		assertEquals(0, BiomeScene.DESERT);
		assertEquals(1, BiomeScene.MUSHROOM);
		assertEquals(2, BiomeScene.LUSH);
		assertEquals(3, BiomeScene.DRIPSTONE);
		assertEquals(4, BiomeScene.DEEP_DARK);
		assertEquals(5, BiomeScene.SNOW);
		assertEquals(6, BiomeScene.MANGROVE);
	}
}
