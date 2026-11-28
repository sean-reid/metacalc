; A LIB_WINDOW byte window over the library for walking a record: lr_win holds lr_wlen bytes
; from the absolute offset lr_wbase, and lr_wp is the cursor in it.

; in: A:HL = absolute offset. Puts the cursor there, inside the window if it holds the byte.
lw_seek:
	ld (lr_ent),hl
	ld (lr_ent+2),a
	ld de,(lr_wbase)
	or a
	sbc hl,de
	ld a,(lr_ent+2)
	ld b,a
	ld a,(lr_wbase+2)
	ld c,a
	ld a,b
	sbc a,c
	jr c,lws_miss
	or h
	jr nz,lws_miss
	ld a,(lr_wlen)
	ld c,a
	ld a,l
	cp c
	jr nc,lws_miss
	ld (lr_wp),a
	ret
lws_miss:
	ld hl,(lr_ent)
	ld (lr_wbase),hl
	ld a,(lr_ent+2)
	ld (lr_wbase+2),a
	xor a
	ld (lr_wlen),a
	ld (lr_wp),a
	ret

; out: A:HL = the absolute offset of the cursor.
lw_pos:
	ld a,(lr_wp)
	ld e,a
	ld d,0
	ld hl,(lr_wbase)
	add hl,de
	ld a,(lr_wbase+2)
	adc a,0
	ret

; in: BC = bytes to move the cursor on.
lw_skip:
	ld a,b
	or a
	jr nz,lws_far
	ld hl,lr_wp
	ld a,(lr_wlen)
	sub (hl)
	cp c
	jr c,lws_far
	ld a,(hl)
	add a,c
	ld (hl),a
	ret
lws_far:
	push bc
	call lw_pos
	pop bc
	call lr_adv
	jr lw_seek

; out: HL = the window byte under the cursor.
lw_ptr:
	ld a,(lr_wp)
	ld l,a
	ld h,0
	ld de,lr_win
	add hl,de
	ret

; in: A = bytes wanted from the cursor, at most LIB_WINDOW. Refills the window at the cursor if it
; holds fewer. out: Z, or NZ and A = LIB_ERR_BAD when the library ends first.
; Held: the window already holds that many from the cursor and is left as it is. Destroys A, C, HL and
; the flags.
; Refilled: the window is refilled from the cursor, as far as LIB_WINDOW and the library allow, with
; lr_wp 0. Destroys all but IX, IY.
; Refused: the window is empty at the cursor (lr_wlen 0, lr_wp 0). The length checks refuse before any
; read starts; a read that fails refuses the same way with lr_wlen already 0. Destroys all but IX, IY.
lw_need:
	ld c,a
	ld a,(lr_wlen)
	ld hl,lr_wp
	sub (hl)
	cp c
	jr c,lwn_fill
	xor a
	ret
lwn_fill:
	ld e,(hl)
	ld d,0
	ld hl,(lr_wbase)
	add hl,de
	ld a,(lr_wbase+2)
	adc a,0
	ld (lr_wbase),hl
	ld (lr_wbase+2),a
	xor a
	ld (lr_wlen),a
	ld (lr_wp),a
	ld de,(lr_wbase)
	ld hl,(lr_length)
	or a
	sbc hl,de
	ld a,(lr_wbase+2)
	ld b,a
	ld a,(lr_length+2)
	sbc a,b
	jp c,lr_bad
	or h
	ld a,LIB_WINDOW
	jr nz,lwn_take
	ld a,l
	cp LIB_WINDOW
	jr c,lwn_take
	ld a,LIB_WINDOW
lwn_take:
	cp c
	jp c,lr_bad
	ld (lr_cnt),a
	ld c,a
	ld b,0
	ld de,lr_win
	ld hl,(lr_wbase)
	ld a,(lr_wbase+2)
	call lr_rd
	ret nz
	ld a,(lr_cnt)
	ld (lr_wlen),a
	xor a
	ret
