package io.github.cumsauce.foley.api;

/** A live sound override registration, from {@link SoundOverrideRegistry#register}. */
public interface SoundOverrideRegistration {
	/** Removes this override. Safe to call more than once, and after {@link SoundOverrideRegistry#clear()}. */
	void unregister();
}
