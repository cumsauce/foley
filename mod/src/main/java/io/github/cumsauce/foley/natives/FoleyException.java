package io.github.cumsauce.foley.natives;

/** A native call failed. {@code code} is one of the {@code FOLEY_ERR_*} values in {@code foley.h}. */
public final class FoleyException extends RuntimeException {
	public static final int INVALID_ARGUMENT = -1;
	public static final int ABI_MISMATCH = -2;
	public static final int PANIC = -3;

	private final int code;

	public FoleyException(int code, String message) {
		super(message + " (error " + code + ")");
		this.code = code;
	}

	public int code() {
		return code;
	}
}
