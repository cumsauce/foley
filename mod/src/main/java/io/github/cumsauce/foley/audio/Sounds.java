package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.sounds.SoundSource;

/** Foley's sound events ({@code foley:<name>} in sounds.json), made once each, and a way to play them in the world. */
public final class Sounds {
	private static final Map<String, SoundEvent> EVENTS = new ConcurrentHashMap<>();

	private Sounds() {}

	/** The event {@code foley:<name>}. Any thread. */
	public static SoundEvent event(String name) {
		return EVENTS.computeIfAbsent(name,
				key -> SoundEvent.createVariableRangeEvent(Identifier.fromNamespaceAndPath("foley", key)));
	}

	/**
	 * Plays {@code foley:<name>} at ({@code x}, {@code y}, {@code z}), at {@code volume} times Foley's own, unless this
	 * tick's share of one-shots is spent. Returns whether it played. Client thread.
	 */
	public static boolean play(Minecraft minecraft, String name, SoundSource source, double x, double y, double z,
			float volume, float pitch) {
		if (!FoleyClient.oneShot()) return false;
		minecraft.getSoundManager().play(new SimpleSoundInstance(event(name), source, volume * FoleyClient.gain(), pitch,
				SoundInstance.createUnseededRandom(), x, y, z));
		return true;
	}
}
