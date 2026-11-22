; The step of the check job, which check.inc includes: its contract is there.
check_step:
	ld a, (ck_state)
	cp CK_S_READY
	ld a, CK_BAD_STATE
	ret nz
	ld hl, (ck_counts)
	ld a, h
	or l
	jr z, cs_done
	ld hl, (ck_i)
	ld de, (ck_n)
	call store_next_zero
	or a
	jp nz, ck_stop
	ld de, (ck_n)
	ld (ck_i), hl
	or a
	sbc hl, de
	jr nc, cs_done
	ld hl, (ck_i)
	call ck_fast
	call far_kernel_verify
	or a
	jr z, cs_passed
	ld b, a
	call ck_slow
	ld a, b
	cp ST_INTERRUPTED
	jr z, cs_paused
	ld a, (ck_have_first)
	or a
	jr nz, cs_failed
	inc a
	ld (ck_have_first), a
	ld a, b
	ld (ck_first_st), a
	ld hl, (ck_i)
	ld (ck_first), hl
cs_failed:
	ld b, 2
	jr cs_write
cs_done:
	ld a, CK_DONE
	ret
cs_paused:
	ld a, CK_PAUSED
	ret
cs_passed:
	call far_kernel_refs
	cp ST_INTERRUPTED
	jr nz, cs_listed
	call far_kernel_refs
cs_listed:
	ld (ck_roff), hl
	ld (ck_rleft), bc
	ld b, a
	ld a, e
	ld (ck_roff + 2), a
	call ck_slow
	ld a, b
	cp ST_INTERRUPTED
	jr z, cs_paused
	or a
	jp nz, cs_kernel
cs_piece:
	ld bc, (ck_rleft)
	ld a, b
	or c
	jr z, cs_ones
	ld hl, CK_PIECE
	or a
	sbc hl, bc
	jr nc, cs_take
	ld bc, CK_PIECE
cs_take:
	ld hl, (ck_rleft)
	or a
	sbc hl, bc
	ld (ck_rleft), hl
	ld h, b
	ld l, c
	add hl, hl
	ld b, h
	ld c, l
	push bc
	ld a, (ck_roff + 2)
	ld hl, (ck_roff)
	ld de, CK_BUF
	call far_lib_read
	pop bc
	or a
	jr nz, cs_reader
	ld hl, (ck_roff)
	add hl, bc
	ld (ck_roff), hl
	jr nc, cs_same_chunk
	ld hl, ck_roff + 2
	inc (hl)
cs_same_chunk:
	srl b
	rr c
	ld hl, CK_BUF
	call store_all_ones
	or a
	jp nz, ck_stop
	ld a, b
	or a
	jr nz, cs_piece
	ld b, 3
	jr cs_write
cs_ones:
	ld b, 1
cs_write:
	ld hl, (ck_i)
	push bc
	call store_set
	pop bc
	or a
	jp nz, ck_stop
	ld hl, (ck_counts)
	dec hl
	ld (ck_counts), hl
	ld a, b
	add a, a
	ld e, a
	ld d, 0
	ld hl, ck_counts
	add hl, de
	inc (hl)
	jr nz, cs_counted
	inc hl
	inc (hl)
cs_counted:
	ld hl, (ck_i)
	inc hl
	ld (ck_i), hl
	ld hl, (ck_counts)
	ld a, h
	or l
	ld a, CK_MORE
	ret nz
	ld a, CK_DONE
	ret
cs_kernel:
	ld b, a
	ld a, CK_S_STOPPED
	ld (ck_state), a
	ld a, CK_REFUSED
	ret
cs_reader:
	ld b, a
	ld a, CK_S_STOPPED
	ld (ck_state), a
	ld a, CK_READER
	ret
