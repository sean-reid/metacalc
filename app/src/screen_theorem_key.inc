; The theorem screen's key (screen_theorem.inc has the kind's header): the model's press over fm_step, fm_folded,
; fm_fold, fm_nodes and fm_span.

#ifdef LIBRARY
; In: A = the key, HL = the frame.
theorem_key:
	ld c, a
	ld a, (TV_LARGE)
	or a
	jr z, tk_main
	ld a, c
	cp KEY_F2
	jr z, tk_notice
	cp KEY_F4
	jr z, tk_notice
	cp KEY_CLEAR
	jr z, tk_pop
	jr tk_stay
tk_main:
	ld a, c
	cp KEY_CLEAR
	jr z, tk_pop
	cp KEY_F2
	jr c, tk_work
	cp KEY_F5
	jr c, tk_notice
tk_stay:
	ld a, SCR_STAY
	ret
tk_pop:
	ld a, SCR_POP
	ret
tk_notice:
	inc hl
	ld c, (hl)
	ld b, NOTICE_LIST
	ld a, KIND_NOTICE
	jp screen_push_param
tk_work:
	ld a, c
	or a
	jr z, tk_stay
	ld (TV_KEY), a
	ld (TV_FRAME), hl
	ld de, 4
	add hl, de
	ld de, TV_S
	ld bc, 5
	ldir
	ld hl, TV_S
	ld de, TV_OLD
	ld bc, 4
	ldir
	ld a, 1
	ld (TV_DOFOLLOW), a
	xor a
	ld (TV_CHG), a
	ld a, (TV_S)
	call theorem_select
	jp nz, tk_fail
	ld a, (TV_KEY)
	cp KEY_ENTER
	jp z, tk_fold
	cp KEY_F1
	jp z, tk_fold
	cp KEY_UP
	jp z, tk_up
	cp KEY_DOWN
	jp z, tk_down
	cp KEY_LEFT
	ld a, 3
	jr z, tk_sibling
	ld a, 2
tk_sibling:
	ld (TV_ROW), a
	ld a, (TV_N)
	ld e, a
	ld a, (TV_ROW)
	ld d, a
	call far_fm_step
	or a
	jp nz, tk_fail
	ld a, b
	inc a
	jr z, tk_none
	dec a
	ld (TV_N), a
	jp tk_done
tk_none:
	ld a, (TV_N)
	or a
	jp nz, tk_done
	ld a, (TV_ROW)
	cp 3
	ld a, (TV_S)
	jr z, tk_before
	ld hl, TV_NHYPS
	cp (hl)
	jp nc, tk_done
	inc a
	jr tk_to
tk_before:
	or a
	jp z, tk_done
	dec a
tk_to:
	ld (TV_S), a
	jp tk_done
tk_fail:
	ld c, a
	ld b, NOTICE_ERROR
	ld a, KIND_NOTICE
	jp screen_push_param
tk_fold:
	ld a, (TV_N)
	ld e, a
	ld d, 2
	call far_fm_fold
	or a
	jp nz, tk_fail
	ld a, b
	ld (TV_CHG), a
	or a
	jp z, tk_done
	call theorem_place
	jp nz, tk_fail
	jp tk_done
tk_down:
	ld a, (TV_N)
	ld e, a
	call far_fm_folded
	or a
	jp nz, tk_fail
	bit 0, b
	jr nz, tk_one
	ld a, (TV_N)
	ld e, a
	ld d, 1
	call far_fm_step
	or a
	jp nz, tk_fail
	ld a, b
	inc a
	jr z, tk_one
	dec a
	ld (TV_N), a
	jp tk_done
tk_one:
	call far_fm_nodes
	or a
	jp nz, tk_fail
	ld a, b
	dec a
	jp nz, tk_done
	ld (TV_DOFOLLOW), a
	ld hl, (TV_SCROLL)
	ld de, THM_ROWS
	add hl, de
	ld de, (TV_TOTAL)
	or a
	sbc hl, de
	jp nc, tk_done
	ld hl, (TV_SCROLL)
	inc hl
	ld (TV_SCROLL), hl
	jp tk_done
tk_up:
	ld a, (TV_N)
	ld e, a
	ld d, 0
	call far_fm_step
	or a
	jp nz, tk_fail
	ld a, b
	inc a
	jr z, tk_back
	dec a
	ld (TV_N), a
	jp tk_done
tk_back:
	call far_fm_nodes
	or a
	jp nz, tk_fail
	ld a, b
	dec a
	jp nz, tk_done
	ld (TV_DOFOLLOW), a
	ld hl, (TV_SCROLL)
	ld a, h
	or l
	jp z, tk_done
	dec hl
	ld (TV_SCROLL), hl
tk_done:
	call theorem_finish
	jp nz, tk_fail
	call theorem_store
	ld a, (TV_CHG)
	or a
	jr nz, tk_changed
	ld hl, TV_S
	ld de, TV_OLD
	ld b, 4
tk_cmp:
	ld a, (de)
	cp (hl)
	jr nz, tk_changed
	inc hl
	inc de
	djnz tk_cmp
	ld a, SCR_STAY
	ret
tk_changed:
	ld a, SCR_REDRAW
	ret
#else
theorem_key:
	xor a
	ret
#endif
