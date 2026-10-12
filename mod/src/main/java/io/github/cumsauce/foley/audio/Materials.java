package io.github.cumsauce.foley.audio;

import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.util.HashMap;
import java.util.IdentityHashMap;
import java.util.Locale;
import java.util.Map;
import net.minecraft.tags.BlockTags;
import net.minecraft.world.level.block.LeavesBlock;
import net.minecraft.world.level.block.SoundType;
import net.minecraft.world.level.block.state.BlockState;
import org.jspecify.annotations.Nullable;

/**
 * What a block is made of, as the native modal synthesis takes it ({@code Material}'s order): from the block's tags
 * where they say (ice, leaves and plants, dirt, sand, wool and snow), else from its {@link SoundType}, which the game
 * already picks by material. Sound types are told apart by their names (the game is unobfuscated), read once.
 */
public final class Materials {
	public static final int STONE = 0;
	public static final int WOOD = 1;
	public static final int METAL = 2;
	public static final int GLASS = 3;
	public static final int ICE = 4;
	public static final int EARTH = 5;
	public static final int GRANULAR = 6;
	public static final int PLANT = 7;
	public static final int SOFT = 8;
	public static final String[] NAMES = {"stone", "wood", "metal", "glass", "ice", "earth", "granular", "plant", "soft"};

	private static final Map<SoundType, Integer> BY_SOUND = new IdentityHashMap<>();
	private static final Map<String, SoundType> BY_EVENT = new HashMap<>();

	static {
		for (Field field : SoundType.class.getFields()) {
			if (!Modifier.isStatic(field.getModifiers()) || field.getType() != SoundType.class) continue;
			try {
				SoundType type = (SoundType) field.get(null);
				BY_SOUND.putIfAbsent(type, byName(field.getName()));
				for (var event : new net.minecraft.sounds.SoundEvent[] {type.getBreakSound(), type.getHitSound(),
						type.getPlaceSound(), type.getFallSound(), type.getStepSound()}) {
					BY_EVENT.putIfAbsent(event.location().toString(), type);
				}
			} catch (IllegalAccessException e) {
				throw new IllegalStateException(e);
			}
		}
	}

	private Materials() {}

	/** {@code state}'s material. */
	public static int of(BlockState state) {
		if (state.is(BlockTags.ICE)) return ICE;
		if (state.getBlock() instanceof LeavesBlock || state.is(BlockTags.LEAVES) || state.is(BlockTags.CROPS)
				|| state.is(BlockTags.FLOWERS) || state.is(BlockTags.SAPLINGS)
				|| state.is(BlockTags.REPLACEABLE_BY_TREES)) {
			return PLANT;
		}
		if (state.is(BlockTags.DIRT)) return EARTH;
		if (state.is(BlockTags.SAND)) return GRANULAR;
		if (state.is(BlockTags.WOOL) || state.is(BlockTags.WOOL_CARPETS) || state.is(BlockTags.SNOW)) return SOFT;
		return of(state.getSoundType());
	}

	public static int of(SoundType type) {
		return BY_SOUND.getOrDefault(type, STONE);
	}

	/** The sound type a block sound event (break, hit, place, fall, step) belongs to, or null. */
	public static @Nullable SoundType soundTypeOf(String event) {
		return BY_EVENT.get(event);
	}

	static int byName(String name) {
		String n = name.toUpperCase(Locale.ROOT);
		if (has(n, "GLASS", "AMETHYST")) return GLASS;
		if (has(n, "COPPER", "IRON", "METAL", "ANVIL", "NETHERITE", "CHAIN", "LANTERN", "HEAVY_CORE", "VAULT",
				"TRIAL_SPAWNER")) {
			return METAL;
		}
		if (has(n, "WOOD", "PLANK", "BAMBOO", "STEM", "LADDER", "SCAFFOLDING", "BOOKSHELF", "HANGING_SIGN")) return WOOD;
		if (has(n, "GRAVEL", "SAND", "SOUL_SOIL", "SUSPICIOUS")) return GRANULAR;
		if (has(n, "WOOL", "SNOW", "SLIME", "HONEY", "CARPET", "SPONGE", "FROGSPAWN")) return SOFT;
		if (has(n, "MUD", "DIRT", "CLAY", "FARMLAND", "MOSS")) return EARTH;
		if (has(n, "GRASS", "LEAVES", "AZALEA", "VINE", "CROP", "BERRY", "WART", "LILY", "FLOWER", "ROOTS", "FUNGUS",
				"SPORE", "DRIPLEAF", "PETALS", "SAPLING", "HANGING_MOSS", "LEAF_LITTER", "BUSH", "CACTUS")) {
			return PLANT;
		}
		return STONE;
	}

	private static boolean has(String name, String... parts) {
		for (String part : parts) {
			if (name.contains(part)) return true;
		}
		return false;
	}
}
