package io.github.cumsauce.foley.audio;

import com.mojang.blaze3d.audio.SoundBuffer;
import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.api.FoleyMaterials;
import io.github.cumsauce.foley.api.MaterialDescriptor;
import io.github.cumsauce.foley.natives.NativeBridge;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.ArrayList;
import java.util.IdentityHashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicInteger;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.tags.TagKey;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.SoundType;
import net.minecraft.world.level.block.state.BlockState;
import org.jspecify.annotations.Nullable;

/**
 * Other mods' own materials (F1): a {@link MaterialDescriptor} under an id, with blocks mapped onto it by exact
 * block, tag or sound type. Foley's layers ({@link Layers}) ask here before the built-in ten, and the layer's buffer
 * is rendered through {@code foley_render_contact_custom} like any generated one-shot, under
 * {@code foley:generated/custom_<index>_<contact>_<variant>}.
 */
public final class CustomMaterials {
	private static final Map<Identifier, Defined> DEFINED = new LinkedHashMap<>();
	private static final Map<Block, Identifier> BLOCKS = new IdentityHashMap<>();
	private static final List<Mapped<TagKey<Block>>> TAGS = new ArrayList<>();
	private static final Map<SoundType, Identifier> SOUND_TYPES = new IdentityHashMap<>();
	/** The packs' own mappings, replaced at each resource reload. */
	private static volatile Map<Block, Identifier> PACK_BLOCKS = Map.of();
	private static volatile List<Mapped<TagKey<Block>>> PACK_TAGS = List.of();
	private static boolean frozen;
	private static final AtomicInteger RENDERED = new AtomicInteger();

	private record Defined(MaterialDescriptor material, int index, String origin) {}

	private record Mapped<T>(T what, Identifier material) {}

	private CustomMaterials() {}

	/** Whether a custom material has this id. */
	public static boolean isDefined(Identifier id) {
		return DEFINED.containsKey(id);
	}

	/**
	 * The custom material {@code state} is made of: its exact block, then a tag of its — an addon's mappings first,
	 * a pack's after. Null if none.
	 */
	static @Nullable Identifier of(BlockState state) {
		Identifier byBlock = BLOCKS.get(state.getBlock());
		if (byBlock != null) return byBlock;
		byBlock = PACK_BLOCKS.get(state.getBlock());
		if (byBlock != null) return byBlock;
		for (Mapped<TagKey<Block>> tag : TAGS) {
			if (state.is(tag.what())) return tag.material();
		}
		for (Mapped<TagKey<Block>> tag : PACK_TAGS) {
			if (state.is(tag.what())) return tag.material();
		}
		return null;
	}

	/** The custom material a block with this sound group is made of, or null. */
	static @Nullable Identifier of(SoundType type) {
		return SOUND_TYPES.get(type);
	}

	/**
	 * The layer's sound name for {@code custom} and a contact ({@code Layers.CONTACTS}' index): which one it is
	 * ({@code GeneratedSounds} resolves it), or null if the id was never defined.
	 */
	static @Nullable String layerName(Identifier custom, int contact) {
		Defined defined = DEFINED.get(custom);
		return defined == null ? null : "custom_" + defined.index() + "_" + contact;
	}

	/** Renders one variant of custom material {@code index}'s {@code contact}, or null if there is no such material. */
	public static short @Nullable [] render(NativeBridge bridge, int index, int contact, int variant) {
		for (Defined defined : DEFINED.values()) {
			if (defined.index() != index) continue;
			float speed = switch (contact) {
				case 1 -> 0.8f; // A block set down.
				case 2 -> 5f; // A body landing.
				case 4 -> 1f; // A heel.
				default -> 3f; // A swung tool; a break takes no speed, but it's checked.
			};
			short[] pcm = bridge.renderContactCustom(defined.material().toFloats(), contact, speed,
					0xB007L * (index + 1) + 31L * contact + variant);
			RENDERED.incrementAndGet();
			return pcm;
		}
		return null;
	}

	/** How many custom-material buffers have been rendered. For the client gametest. */
	public static int rendered() {
		return RENDERED.get();
	}

	/** The registry, as one addon sees it: every change names its addon in any error. */
	static FoleyMaterials api(String origin) {
		return new FoleyMaterials() {
			@Override
			public void register(Identifier id, MaterialDescriptor material) {
				if (frozen) throw new IllegalStateException(origin + " registered " + id + " after its entrypoint returned");
				if (!origin.equals(id.getNamespace()))
					throw new IllegalArgumentException(origin + " may not define " + id + ": a material's namespace is its owner's");
				if (DEFINED.containsKey(id))
					throw new IllegalArgumentException(id + " is already a material");
				if (material == null) throw new IllegalArgumentException(id + " needs a descriptor");
				DEFINED.put(id, new Defined(material, DEFINED.size(), origin));
			}

			@Override
			public void mapBlock(Block block, Identifier material) {
				BLOCKS.put(block, check(origin, "block", material));
			}

			@Override
			public void mapTag(TagKey<Block> tag, Identifier material) {
				TAGS.add(new Mapped<>(tag, check(origin, "tag", material)));
			}

			@Override
			public void mapSoundType(SoundType type, Identifier material) {
				SOUND_TYPES.put(type, check(origin, "sound type", material));
			}
		};
	}

	/** A mapping's material must be one of this addon's own. */
	private static Identifier check(String origin, String what, Identifier material) {
		Defined defined = DEFINED.get(material);
		if (defined == null || !defined.origin().equals(origin))
			throw new IllegalArgumentException(origin + " mapped a " + what + " onto " + material
					+ ", which is not one of its materials");
		return material;
	}

	/** Closes registration: every entrypoint has returned. */
	static void freeze() {
		frozen = true;
		FoleyClient.LOGGER.info("Foley: {} custom material(s)", DEFINED.size());
	}

	/**
	 * A pack's materials, read at a resource reload: {@code materials} (name to descriptor), and its mappings,
	 * {@code blocks} and {@code tags} (block or tag id to material name). Names already defined in code stay theirs;
	 * the pack's mappings replace the last reload's.
	 */
	public static void fromPacks(String namespace, Map<String, MaterialDescriptor> materials,
			Map<String, String> blocks, Map<String, String> tags) {
		Map<String, Identifier> names = new ConcurrentHashMap<>();
		materials.forEach((name, material) -> {
			if (name.matches("[a-z0-9_]+")) {
				Identifier id = Identifier.fromNamespaceAndPath(namespace, name);
				if (!DEFINED.containsKey(id)) DEFINED.put(id, new Defined(material, DEFINED.size(), namespace));
				names.put(name, id);
			}
		});
		Map<Block, Identifier> packBlocks = new IdentityHashMap<>();
		List<Mapped<TagKey<Block>>> packTags = new ArrayList<>();
		blocks.forEach((block, name) -> {
			Identifier id = names.get(name);
			Identifier blockId = Identifier.tryParse(block);
			if (id != null && blockId != null) {
				Block known = BuiltInRegistries.BLOCK.getValue(blockId);
				packBlocks.put(known, id);
			}
		});
		tags.forEach((tag, name) -> {
			Identifier id = names.get(name);
			Identifier tagId = Identifier.tryParse(tag);
			if (id != null && tagId != null) {
				packTags.add(new Mapped<>(TagKey.create(net.minecraft.core.registries.Registries.BLOCK, tagId), id));
			}
		});
		PACK_BLOCKS = packBlocks;
		PACK_TAGS = List.copyOf(packTags);
	}
}
