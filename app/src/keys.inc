; The app's one key wait: app_key waits for a key with the OS's _GetKey, sets the canvas font
; again, and returns the model's key (crates/mcui/src/key.rs). Every call into the OS that waits
; for a key is here; app_key_waiting, below it, does not wait.
;
; Needs equates.inc before it. It loads the address ui_font and calls KEYS_SET_FONT, which is
; ui_set_font unless the including file defines another routine first: the one that reaches
; ui_set_font from another page, with ui_font on the page ui_set_font runs on.
;
; app_key   out: A = the key, one of the KEY_ codes below; KEY_NONE for a key the app ignores
;                B = for KEY_LETTER, the character as ASCII, uppercase for a letter; else 0
;                destroys every register but IY, which must hold the OS flags pointer; the
;                shadow registers and IX are the OS's to change. Returns with interrupts on.
;
; KEY_LETTER carries the characters the model's list accepts that a key face has: A to Z, 0 to
; 9, '-' and '.'; the model also takes '_', which no key carries.
;   Plain: a key that carries a letter gives it, so the list's jump is one press and the eight
;   digit keys 1 2 4 5 6 7 8 9 and the subtract key give their letters; the 3, 0 and dot keys
;   carry none and give their character.
;   After ALPHA, and under alpha lock: the keys 0 to 9, subtract and dot give the digit or the
;   '-' or '.' on their face. Every other key that carries a letter (MATH APPS PRGM x^-1 SIN COS
;   TAN ^ x^2 , ( ) / LOG x LN STO) gives KEY_NONE, as do (-), +, VARS, STAT, DEL, X,T,theta,n
;   and MODE. The arrows, ENTER, CLEAR and the five keys under the screen are the same as plain.
; 2nd and ALPHA return nothing. After 2nd every key gives KEY_NONE except CLEAR, which the OS
; reports as Clear and gives KEY_CLEAR, and MODE, which is QUIT and gives KEY_QUIT.
;
; ON is never returned. The OS sets onInterrupt when ON goes down and _GetKey then returns 0 at
; once, on every call, until the flag is clear. app_key waits for ON to come up with interrupts
; off for the read and the clear, clears the flag and waits for a key again. A press that lands
; after the clear sets the flag again and is taken the same way by the next wait. The shell clears
; the flag at every idle point that reports ON, sets APP_ON to 1 and goes on to scr_idle, where a
; screen's idle routine may read and clear it: the checking screen's does and stops. APP_ON is 0 at
; the start and after every key app_key returns. A press made between waits reaches only a routine
; that polls the flag or APP_ON.
;
; The OS copies the screen's pixels over UI_VARS when auto power down fires inside _GetKey, and
; the font pointer, ui_dirty and ui_sent are the only things in it that hold state between calls; app_key sets the
; canvas font after every wait, power down or not, and the redraw after one is in full, its ui_clear storing $FF in
; ui_dirty. 2nd+OFF never returns: the OS calls cxPPutAway and
; cxPutAway of the context the app installed with _AppInit (app_putaway in metacalc.z80), powers
; off, and does not enter the app when the calculator is turned back on. An auto power down calls
; neither, and the wait goes on after ON.

KEY_NONE         .equ 0
KEY_UP           .equ 1
KEY_DOWN         .equ 2
KEY_LEFT         .equ 3
KEY_RIGHT        .equ 4
KEY_ENTER        .equ 5
KEY_CLEAR        .equ 6
KEY_F1           .equ 7
KEY_F2           .equ 8
KEY_F3           .equ 9
KEY_F4           .equ 10
KEY_F5           .equ 11
KEY_LETTER       .equ 12
KEY_QUIT         .equ 13

KEY_PAIRS        .equ 12
KEY_CHARS        .equ 41

#ifndef KEYS_SET_FONT
#define KEYS_SET_FONT ui_set_font
#endif

app_key:
ak_wait:
	ei
	bcall(_GetKey)
	bit onInterrupt,(iy+onFlags)
	jr nz,ak_on
	ld c,a
	ld hl,ui_font
	call KEYS_SET_FONT
	ld b,0
	ld a,c
	ld hl,ak_pairs
	ld d,KEY_PAIRS
	call ak_find
	jr z,ak_hit
	ld hl,ak_chars
	ld d,KEY_CHARS
	call ak_find
	jr z,ak_char
	xor a
	ret
ak_char:
	ld b,(hl)
	ld a,KEY_LETTER
	ret
ak_hit:
	ld a,(hl)
	ret
ak_on:
	ei
	halt
	di
	in a,(4)
	bit 3,a
	jr z,ak_on
	res onInterrupt,(iy+onFlags)
	jr ak_wait

; The OS holds a key's scan code in kbdScanCode from its press until _GetKey takes it or until about
; 40 ms after the release, and a key held between auto repeats has none.
;
; app_key_waiting   out: Z, carry clear, A = 0: nothing; the port read showed every key and ON up
;                        NZ, carry clear: a scan code is held, A = it; app_key returns that key at
;                        once. A pending 2nd or ALPHA is a code: app_key takes it and waits for the
;                        key after it
;                        NZ, carry set: no scan code. A bit 3 is set when onInterrupt is set or ON is
;                        down, bit 0 when a key is down on the port. When onInterrupt is set the
;                        port is not read and bit 0 is clear
;                   takes nothing, so the next app_key sees the OS's key bytes as they were.
;                   Each exit reads, destroys and leaves the interrupts as follows:
;                   scan code: reads kbdScanCode only; destroys A and the flags; touches no port and
;                     leaves the interrupt state as it was
;                   break flag: reads kbdScanCode and the onInterrupt flag through IY; destroys A
;                     and the flags; touches no port and leaves the interrupt state as it was
;                   port: reads those two, then port 1 with every key group selected and port 4;
;                     leaves port 1 at FFh, every group deselected; destroys A, B, C and the flags;
;                     the reads run with interrupts off and it returns with them on
;                   needs interrupts on at the call for the port exit. The formula routines run with
;                   interrupts off and must not call it
;                   T-states, measured in the emulator from the call to the instruction after
;                   it, so they include the call and the OS's wait states (1 per opcode fetch from
;                   flash, 1 per stack write), the same at 6 and at 15 MHz: 51 on the scan code, 99 on
;                   the break flag, 269 when nothing is down, 279 with ON down, 282 with a key down
;                   whether or not ON is; the ON figures are the stub's, the reference ROM's are not
;                   measured
;                   two push af / pop af pairs (42 T-states) between the group write and the read
;                   let the keypad lines settle at 15 MHz

app_key_waiting:
	ld a,(kbdScanCode)
	or a
	ret nz
	bit onInterrupt,(iy+onFlags)
	jr z,akw_port
	ld a,8
	scf
	ret
akw_port:
	di
	xor a
	out (1),a
	push af
	pop af
	push af
	pop af
	in a,(1)
	ld c,a
	ld a,0FFh
	out (1),a
	in a,(4)
	ei
	cpl
	and 8
	ld b,a
	ld a,c
	inc a
	ld a,b
	jr z,akw_keyless
	or 1
akw_keyless:
	or a
	ret z
	scf
	ret

; app_tick   waits for the next interrupt and returns
;             in: interrupts on or off. out: interrupts on. destroys nothing, the flags included;
;             the shadow registers are the OS's to change
;             stack: 2 bytes, its return address; an interrupt taken in the halt pushes its own on top
;             an interrupt already pending when it is called with interrupts off ends the tick at
;             once; the halt ends on any interrupt the OS has unmasked, and the figures below are
;             for the timer; it returns only while some source is unmasked
;             length: with the app in its loop on the reference ROM, 298 consecutive ticks took
;             55,661 to 55,665 T-states at 6 MHz and 139,152 to 139,156 at 15 MHz, 9.28 ms each, so
;             11 ticks are about 102 ms

app_tick:
	ei
	halt
	ret

; In: A = the OS's code, HL = D pairs of code and value. Out: Z with HL at the value for a hit.
ak_find:
	cp (hl)
	inc hl
	ret z
	inc hl
	dec d
	jr nz,ak_find
	dec d
	ret

; The OS's code for a key and the code it gives.
ak_pairs:
	.db $03,KEY_UP
	.db $04,KEY_DOWN
	.db $02,KEY_LEFT
	.db $01,KEY_RIGHT
	.db $05,KEY_ENTER
	.db kClear,KEY_CLEAR
	.db $49,KEY_F1
	.db $48,KEY_F2
	.db $2E,KEY_F3
	.db $5A,KEY_F4
	.db $44,KEY_F5
	.db $40,KEY_QUIT

; The OS's code and the character: the key that carries each letter, A to Z; the 3, 0 and dot
; keys; then the codes after ALPHA of the digit, subtract and dot keys.
ak_chars:
	.db $32,'A',$2C,'B',$2D,'C',$B6,'D'
	.db $B7,'E',$B9,'F',$BB,'G',$84,'H'
	.db $BD,'I',$8B,'J',$85,'K',$86,'L'
	.db $83,'M',$C1,'N',$95,'O',$96,'P'
	.db $97,'Q',$82,'R',$BF,'S',$92,'T'
	.db $93,'U',$94,'V',$81,'W',$8A,'X'
	.db $8F,'Y',$90,'Z'
	.db $91,'3',$8E,'0',$8D,'.'
	.db $A8,'7',$A9,'8',$AA,'9',$AD,'4'
	.db $AE,'5',$AF,'6',$B2,'1',$B3,'2'
	.db $CC,'3',$99,'0',$C6,'.',$B0,'-'
