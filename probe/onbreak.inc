; Contract: IY is the OS flags pointer. Turns interrupts on, waits for ON to come up with a halt between
; polls, then clears the OS's pending ON break so no later text call quits the app.
on_release_and_clear:
	in a, (04h)
	bit 3, a
	jr nz, on_released
	ei
	halt
on_wait_halted:
	jr on_release_and_clear
on_released:
	res onInterrupt, (iy + onFlags)
	ret
