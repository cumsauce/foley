package io.github.cumsauce.foley.audio;

import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.Vec3;

/**
 * The wind round the listener, sampled from the world.
 *
 * <ul>
 *   <li><b>Speed:</b> a 10 m reference wind from the weather ({@value #CLEAR} m/s clear, {@value #RAIN} in rain,
 *       {@value #STORM} in a thunderstorm), carried to the listener's height above the ground round about by the log
 *       wind profile, U(z) = U₁₀·ln(z/z₀)/ln(10/z₀), with the roughness z₀ of what's round about (open ground or
 *       woods). So it rises on a hill top and drops in a valley or a forest. The listener's own speed (flying, a
 *       boat) adds to it.</li>
 *   <li><b>Exposure:</b> how much of the sky the listener and the columns round them are open to; 0 deep inside.</li>
 *   <li><b>Leaves:</b> how much of the top of what's round about is leaves, to rustle.</li>
 * </ul>
 *
 * No wind in a dimension without a sky (the Nether, the End).
 *
 * @param speed    the mean wind at the ear, m/s
 * @param exposure 0 shut in to 1 in the open
 * @param leaves   0 to 1
 */
public record WindScene(float speed, float exposure, float leaves) {
	public static final WindScene CALM = new WindScene(0, 0, 0);
	static final float CLEAR = 5;
	static final float RAIN = 9;
	static final float STORM = 16;
	/** How far round to look for the ground and for cover, blocks. */
	private static final int RADIUS = 24;
	private static final int STEP = 4;
	/** Roughness lengths, m: open grassland, and woods. */
	private static final double OPEN = 0.03;
	private static final double WOODS = 1.0;

	public static WindScene sample(Minecraft minecraft) {
		ClientLevel level = minecraft.level;
		if (level == null || minecraft.player == null || !level.dimensionType().hasSkyLight()) return CALM;
		float reference = CLEAR + (RAIN - CLEAR) * level.getRainLevel(1f) + (STORM - RAIN) * level.getThunderLevel(1f);
		Vec3 eye = minecraft.player.getEyePosition();
		double ground = 0;
		float weights = 0;
		float open = 0;
		float leaves = 0;
		BlockPos.MutableBlockPos pos = new BlockPos.MutableBlockPos();
		for (int dx = -RADIUS; dx <= RADIUS; dx += STEP) {
			for (int dz = -RADIUS; dz <= RADIUS; dz += STEP) {
				int x = (int) Math.floor(eye.x) + dx;
				int z = (int) Math.floor(eye.z) + dz;
				int top = level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES, x, z);
				int canopy = level.getHeight(Heightmap.Types.MOTION_BLOCKING, x, z);
				float weight = 1f / (1 + (dx * dx + dz * dz) / 64f);
				ground += weight * top;
				weights += weight;
				if (canopy <= eye.y + 1) open += weight;
				if (canopy > top && Surfaces.of(level.getBlockState(pos.set(x, canopy - 1, z))) == Surfaces.LEAVES) {
					leaves += weight;
				}
			}
		}
		ground /= weights;
		open /= weights;
		leaves /= weights;
		// Height above the ground round about, at least the ear's 1.6 m; woods are rough.
		double height = Math.max(1.6, eye.y - ground + 1.6);
		double roughness = OPEN + (WOODS - OPEN) * Math.min(1, 2 * leaves);
		double profile = Math.log(Math.max(height, roughness * 1.5) / roughness) / Math.log(10 / roughness);
		double speed = reference * Math.clamp(profile, 0.1, 2.5);
		// The listener's own motion through the air, blocks a tick to m/s.
		double moving = minecraft.player.getDeltaMovement().length() * 20;
		speed = Math.hypot(speed, moving);
		// Exposure: the ear itself under the sky counts half, the columns round it the rest.
		float sky = level.canSeeSky(BlockPos.containing(eye)) ? 1 : 0;
		float exposure = Math.clamp(0.5f * sky + 0.5f * open, 0, 1);
		return new WindScene((float) Math.min(speed, 60), exposure, Math.clamp(leaves, 0, 1));
	}

	@Override
	public String toString() {
		return String.format(java.util.Locale.ROOT, "WindScene[speed=%.1f m/s, exposure=%.2f, leaves=%.2f]", speed,
				exposure, leaves);
	}
}
