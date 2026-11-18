; Contract: IY is the OS flags pointer. Takes any interrupt already latched, then waits for ON to come
; up with a halt between polls and clears the OS's pending ON break, so no later text call quits the
; app and no latched interrupt sets it again. Returns with interrupts on.
on_release_and_clear:
	ei
	halt
on_wait_halted:
	di
	in a, (04h)
	bit 3, a
	jr z, on_release_and_clear
on_wait_end:
	res onInterrupt, (iy + onFlags)
	ei
	ret
