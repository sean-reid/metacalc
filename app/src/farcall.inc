; The cross-page call. Every code page includes this file at FAR_CALL, so its bytes are the same
; on each page and execution carries on at the next address when port 6 changes the page under it.
;
; In: IX = the target address on page k, A, F, BC, DE, HL as the callee is to see them.
; Out: A, F, BC, DE, HL as the callee left them; IX destroyed; IY, the shadow registers and the
; interrupt state untouched. Page k is flash page base - k, base being what port 6 read when
; far_init ran. The caller's page is restored from what port 6 read on the way in, so calls nest.
; Stack: 4 bytes below the caller's return address. RAM: FAR_VARS, in bank B; none of bank C.
;
;     ld ix, target
;     call far_page1
;
; far_init runs once on the entry page, before anything changes port 6 and before the first far
; call. far_a holds the caller's A from its store until the load after the port 6 write, so an
; interrupt handler may make a far call only if it saves and restores far_a. The callee returns
; with one of the app's code pages mapped, because the return half runs on whichever page is
; mapped then. FAR_CALL is 3 bytes past the entry point, where the entry page keeps the jump to
; its start. A page with no header defines FAR_PAD to be padded up to FAR_CALL.

#include "ram_map.inc"

#ifndef FAR_RAM_DECLARED
#define FAR_RAM_DECLARED
FAR_CALL        .equ 4083h
#define FAR_NEXT 0
#macro FAR_VAR(name, offset, size)
RAM_VAR(name, FAR_VARS, offset, size, FAR_VARS_SIZE, FAR_NEXT, "the far call variables overlap, leave a gap, or run past FAR_VARS_SIZE in ram_map.inc")
#define FAR_NEXT eval(offset + size)
#endmacro
#macro FAR_AT(offset)
RAM_AT(offset, FAR_VARS_SIZE, FAR_NEXT, "FAR_AT must not go back or past FAR_VARS_SIZE in ram_map.inc")
#define FAR_NEXT eval(offset)
#endmacro

FAR_VAR(far_a, $00, 1)
FAR_VAR(far_pg, $01, 3)
#endif

#ifdef FAR_PAD
 .fill FAR_CALL - $, 0
#endif
#if ($ < FAR_CALL) || ($ > FAR_CALL)
 .error "farcall.inc must be included at FAR_CALL"
#endif

far_jump:
	jp (ix)

far_page0:
	ld (far_a), a
	in a, (6)
	push af
	ld a, (far_pg)
	jr far_cross
far_page1:
	ld (far_a), a
	in a, (6)
	push af
	ld a, (far_pg + 1)
	jr far_cross
far_page2:
	ld (far_a), a
	in a, (6)
	push af
	ld a, (far_pg + 2)
far_cross:
	out (6), a
	ld a, (far_a)
	call far_jump
	ex (sp), hl
	push af
	ld a, h
	out (6), a
	pop af
	pop hl
	ret

far_init:
	in a, (6)
	ld (far_pg), a
	dec a
	ld (far_pg + 1), a
	dec a
	ld (far_pg + 2), a
	ret
far_end:
