; Contract: the including file defines putchar (A = character; 0Ah new line, 0Dh back to the
; start of the line, 0Ch clear the screen and home; it may destroy every register),
; includes the kernel and equates.inc, and calls kernchk_run with interrupts on, from the page
; and port 6 value the app was entered with, and with a usable stack. Scratch is in bank B, which
; the kernel restores after every call; the parameter block for kernel_init is its first nine bytes.
; kernchk_run begins with kernel_reset, so no earlier session's ready mark survives into the run.
; Checks the library whose first chunk is on the page below the app: kernel_init, the CRC-32 run
; chunk by chunk, then kernel_verify for assertion 0 to count - 1, with a progress line between
; calls. Stops when a call returns 14, or between calls when ON is down, was pressed since the
; last interrupt acknowledge, or the OS has flagged it. It then waits for ON to come up and
; clears the OS break before printing anything, so the app stays running. IY must be the OS flags
; pointer. Runs at 15 MHz and restores the speed on return. The result stays in scratch and is
; printed last.

KD_PORT_SPEED = 20h
KD_PORT_KEYS  = 04h
KD_ON_LATCH   = 01h
KD_ON_LEVEL   = 08h
KD_MORE       = 0FFh
KD_INTERRUPT  = 14

K_PARAMS     = libPage
#macro KD_VAR(name, offset, size)
RAM_VAR(name, K_PARAMS, offset, size, KD_SCRATCH_SIZE, "the driver's scratch runs past KD_SCRATCH_SIZE in ram_map.inc")
#endmacro

KD_VAR(K_LIMITS, 1, 8)
KD_VAR(K_COUNT, 9, 2)
KD_VAR(K_CHUNKS, 11, 1)
KD_VAR(K_DONE, 12, 2)
KD_VAR(K_ACCEPTED, 14, 2)
KD_VAR(K_FIRST, 16, 2)
KD_VAR(K_FIRST_ST, 18, 1)
KD_VAR(K_HAVE_FIRST, 19, 1)
KD_VAR(K_INIT, 20, 1)
KD_VAR(K_CRC_STATE, 21, 1)
KD_VAR(K_CRC, 22, 4)
KD_VAR(K_CHUNK, 26, 1)
KD_VAR(K_STOPPED, 27, 1)
KD_VAR(K_STOP_AT, 28, 2)
KD_VAR(K_SPEED, 30, 1)
KD_VAR(P_TMP, 31, 2)
KD_VAR(P_TMP2, 33, 1)
KD_VAR(P_STR, 34, 2)
K_END        = K_PARAMS + KD_SCRATCH_SIZE

CRC_NONE    = 0
CRC_OK      = 1
CRC_BAD     = 2
CRC_STOPPED = 3

STOP_NONE   = 0
STOP_CRC    = 1
STOP_VERIFY = 2
STOP_INIT   = 3

kernchk_run:
	call kernel_reset
	call app_find_library
	ld a, 0Ch
	call putchar
	ld hl, K_PARAMS + 1
	ld de, K_PARAMS + 2
	ld bc, K_END - K_PARAMS - 2
	xor a
	ld (hl), a
	ldir
	in a, (KD_PORT_SPEED)
	ld (K_SPEED), a
	ld a, 1
	out (KD_PORT_SPEED), a
	ld hl, K_LIMITS
	ld b, 8
kd_limits:
	ld (hl), 0FFh
	inc hl
	djnz kd_limits

	ld hl, K_PARAMS
	call kernel_init
	ld (K_INIT), a
	or a
	jr z, kd_inited
	cp KD_INTERRUPT
	jr nz, kd_done
kd_init_stopped:
	xor a
	ld (K_INIT), a
	call on_release_and_clear
	ld a, STOP_INIT
	ld (K_STOPPED), a
	jr kd_done
kd_inited:
	call kd_header
	di
	call kd_on_seen
	jr nz, kd_init_stopped
	call kd_crc
	ld a, (K_CRC_STATE)
	cp CRC_OK
	call z, kd_verify
kd_done:
	ld a, (K_SPEED)
	out (KD_PORT_SPEED), a
	jp kd_report

; Reads the assertion count and the number of 16 KB chunks from the header of the library, which
; maps over bank B and its scratch for the length of the read. Returns with interrupts on.
kd_header:
	di
	ld a, (libPage)
	ld b, a
	in a, (7)
	push af
	ld a, b
	out (7), a
	ld hl, (8000h + 0Eh)
	ld de, (8000h + 08h)
	ld a, (8000h + 0Ah)
	ld b, a
	pop af
	out (7), a
	ei
	ld (K_COUNT), hl
	ld hl, 3FFFh
	add hl, de
	ld a, b
	adc a, 0
	add a, a
	add a, a
	ld b, a
	ld a, h
	rlca
	rlca
	and 3
	or b
	ld (K_CHUNKS), a
	ret

; NZ if ON is down, if its interrupt is latched on port 4, or if the OS has set its break flag.
; Interrupts stay off for a whole kernel call, so a tap shorter than the gap between polls leaves
; only the latch.
kd_on_seen:
	in a, (KD_PORT_KEYS)
	xor KD_ON_LEVEL
	and KD_ON_LATCH | KD_ON_LEVEL
	ret nz
	bit onInterrupt, (iy + onFlags)
	ret

kd_crc:
	ld hl, s_title
	call putstr
	call kd_newline
	call kd_show_crc
	call kernel_crc_begin
	or a
	jr nz, kd_crc_bad
kd_crc_next:
	call kd_on_seen
	jr nz, kd_crc_stopped
	call kernel_crc_chunk
	cp KD_MORE
	jr z, kd_crc_more
	or a
	jr nz, kd_crc_bad
	ld (K_CRC), hl
	ld (K_CRC + 2), de
	ld a, CRC_OK
	ld (K_CRC_STATE), a
	ld hl, K_CHUNKS
	ld a, (hl)
	ld (K_CHUNK), a
	call kd_show_crc
	jp kd_newline
kd_crc_more:
	ld hl, K_CHUNK
	inc (hl)
	call kd_show_crc
	jr kd_crc_next
kd_crc_bad:
	ld a, CRC_BAD
	ld (K_CRC_STATE), a
	ret
kd_crc_stopped:
	call on_release_and_clear
	ld a, CRC_STOPPED
	ld (K_CRC_STATE), a
	ld a, STOP_CRC
	ld (K_STOPPED), a
	ld a, (K_CHUNK)
	ld l, a
	ld h, 0
	ld (K_STOP_AT), hl
	ret

kd_verify:
	call kd_show_check
kd_verify_next:
	ld hl, (K_DONE)
	ld de, (K_COUNT)
	or a
	sbc hl, de
	jr nc, kd_verify_end
	call kd_on_seen
	jr nz, kd_verify_stopped
	ld hl, (K_DONE)
	call kernel_verify
	cp KD_INTERRUPT
	jr z, kd_verify_stopped
	or a
	jr z, kd_accepted
	ld c, a
	ld a, (K_HAVE_FIRST)
	or a
	jr nz, kd_advance
	inc a
	ld (K_HAVE_FIRST), a
	ld a, c
	ld (K_FIRST_ST), a
	ld hl, (K_DONE)
	ld (K_FIRST), hl
	jr kd_advance
kd_accepted:
	ld hl, (K_ACCEPTED)
	inc hl
	ld (K_ACCEPTED), hl
kd_advance:
	ld hl, (K_DONE)
	inc hl
	ld (K_DONE), hl
	ld a, l
	and 15
	call z, kd_show_check
	jr kd_verify_next
kd_verify_end:
	jp kd_show_check
kd_verify_stopped:
	call on_release_and_clear
	ld a, STOP_VERIFY
	ld (K_STOPPED), a
	ld hl, (K_DONE)
	ld (K_STOP_AT), hl
	jp kd_show_check

kd_newline:
	ld a, 0Ah
	jp putchar

; Rewrites the current line as the name and a done/total pair.
kd_show_crc:
	ld a, 0Dh
	call putchar
	ld hl, s_crc
	call putstr
	ld a, (K_CHUNK)
	ld l, a
	ld h, 0
	call putdec
	ld a, '/'
	call putchar
	ld a, (K_CHUNKS)
	ld l, a
	ld h, 0
	jp putdec

kd_show_check:
	ld a, 0Dh
	call putchar
	ld hl, s_check
	call putstr
	ld hl, (K_DONE)
	call putdec
	ld a, '/'
	call putchar
	ld hl, (K_COUNT)
	jp putdec

kd_report:
	ld a, 0Ch
	call putchar
	ld hl, s_title
	call putstr
	call kd_newline
	ld a, (K_INIT)
	or a
	jr z, kd_report_crc
	ld hl, s_init
	call putstr
	ld a, (K_INIT)
	ld l, a
	ld h, 0
	jp putdec
kd_report_crc:
	ld a, (K_STOPPED)
	cp STOP_INIT
	ld hl, s_init_stopped
	jr z, kd_report_crc_text
	ld hl, s_crc
	call putstr
	ld a, (K_CRC_STATE)
	cp CRC_OK
	jr z, kd_report_sum
	cp CRC_STOPPED
	ld hl, s_bad
	jr nz, kd_report_crc_text
	ld hl, s_stopped
kd_report_crc_text:
	call putstr
	jr kd_report_counts
kd_report_sum:
	ld hl, (K_CRC + 2)
	call puthex
	ld hl, (K_CRC)
	call puthex
kd_report_counts:
	call kd_newline
	ld hl, s_checked
	call putstr
	ld hl, (K_DONE)
	call putdec
	call kd_newline
	ld hl, s_accepted
	call putstr
	ld hl, (K_ACCEPTED)
	call putdec
	ld a, (K_HAVE_FIRST)
	or a
	jr z, kd_report_stop
	call kd_newline
	ld hl, s_first
	call putstr
	ld hl, (K_FIRST)
	call putdec
	call kd_newline
	ld hl, s_status
	call putstr
	ld a, (K_FIRST_ST)
	ld l, a
	ld h, 0
	call putdec
kd_report_stop:
	ld a, (K_STOPPED)
	or a
	ret z
	call kd_newline
	ld hl, s_stopped_at
	call putstr
	ld hl, (K_STOP_AT)
	jp putdec

s_title:
	.db "kernchk v", '0' + VERSION, 0
s_crc:
	.db "crc ", 0
s_check:
	.db "chk ", 0
s_init:
	.db "init st ", 0
s_init_stopped:
	.db "init stopped", 0
s_bad:
	.db "bad", 0
s_stopped:
	.db "stopped", 0
s_checked:
	.db "checked ", 0
s_accepted:
	.db "accepted ", 0
s_first:
	.db "first ", 0
s_status:
	.db "status ", 0
s_stopped_at:
	.db "stopped ", 0

#include "printnum.inc"
#include "onbreak.inc"
