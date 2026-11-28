; pw_find inside its session: the goal at a place in reading order, found by walking down from the
; root over the subtree sizes of the goal table. Nothing it reads is written.

; Stores in the block at DE the two bytes of the row of `col` that pv_t2 (twice the goal's number)
; names.
#macro PW_OUT2(col)
	ld hl,(pv_t2)
	ld bc,col
	add hl,bc
	ldi
	ldi
#endmacro

; in: A = a goal's number. out: Z, HL = where its links start in the links table, A = how many it
; has; or NZ when the number is not a goal's or the table is not one a walk makes.
pw_range:
	ld c,a
	ld hl,pv_goals
	cp (hl)
	jr nc,pwq_bad
	ld l,a
	ld h,0
	add hl,hl
	ld de,PW_LSTART
	add hl,de
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	ld a,(pv_goals)
	dec a
	cp c
	jr z,pwq_last
	ld c,(hl)
	inc hl
	ld b,(hl)
	jr pwq_count
pwq_last:
	ld bc,(pv_links)
pwq_count:
	ld h,b
	ld l,c
	or a
	sbc hl,de
	jr c,pwq_bad
	ld a,h
	or a
	jr nz,pwq_bad
	ld a,l
	cp PW_MAX_SUBGOALS + 1
	jr nc,pwq_bad
	ex de,hl
	cp a
	ret
pwq_bad:
	or 1
	ret

pw_find_run:
	ld hl,(lr_tr)
	ld (pf_place),hl
	ld a,(lr_tr+2)
	ld (pf_place+2),a
	or h
	or l
	jr z,pwf_place
	ld hl,(pv_total)
	ld de,(pf_place)
	or a
	sbc hl,de
	ld a,(pv_total+2)
	ld hl,pf_place+2
	sbc a,(hl)
	jr nc,pwf_start
pwf_place:
	ld a,PW_ERR_PLACE
	jp pw_leave
pwf_start:
	ld a,(pv_goals)
	dec a
	ld (pf_cur),a
	ld hl,1
	ld (pf_cp),hl
	ld hl,0
	ld (pf_parent),hl
	xor a
	ld (pf_cp+2),a
	ld (pf_parent+2),a
	ld (pf_depth),a
	ld (pf_idx),a
pwf_level:
	ld hl,(pf_place)
	ld de,(pf_cp)
	or a
	sbc hl,de
	jr nz,pwf_inside
	ld a,(pf_place+2)
	ld hl,pf_cp+2
	cp (hl)
	jp z,pwf_found
pwf_inside:
	ld a,(pf_cur)
	call pw_range
	jp nz,pwf_corrupt
	ld (pf_ls),hl
	ld (pf_n),a
	xor a
	ld (pf_c),a
	ld hl,(pf_cp)
	ld de,1
	add hl,de
	ld a,(pf_cp+2)
	adc a,0
	ld (pf_cs),hl
	ld (pf_cs+2),a
pwf_child:
	ld a,(pf_c)
	ld hl,pf_n
	cp (hl)
	jp nc,pwf_corrupt
	ld hl,(pf_ls)
	ld e,a
	ld d,0
	add hl,de
	ld de,PW_LINKS
	add hl,de
	ld a,(hl)
	ld (pf_g),a
	ld hl,pv_goals
	cp (hl)
	jp nc,pwf_corrupt
	call pw_sizeptr
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	ld b,(hl)
	ld hl,(pf_cs)
	add hl,de
	ld a,(pf_cs+2)
	adc a,b
	ld (pf_ce),hl
	ld (pf_ce+2),a
	ld hl,(pf_place)
	ld de,(pf_ce)
	or a
	sbc hl,de
	ld a,(pf_place+2)
	ld hl,pf_ce+2
	sbc a,(hl)
	jr c,pwf_down
	ld hl,(pf_ce)
	ld (pf_cs),hl
	ld a,(pf_ce+2)
	ld (pf_cs+2),a
	ld hl,pf_c
	inc (hl)
	jr pwf_child
pwf_down:
	ld hl,(pf_cp)
	ld (pf_parent),hl
	ld a,(pf_cp+2)
	ld (pf_parent+2),a
	ld a,(pf_c)
	ld (pf_idx),a
	ld a,(pf_g)
	ld (pf_cur),a
	ld hl,(pf_cs)
	ld (pf_cp),hl
	ld a,(pf_cs+2)
	ld (pf_cp+2),a
	ld a,(pf_depth)
	inc a
	jp z,pwf_corrupt
	ld (pf_depth),a
	jp pwf_level
pwf_corrupt:
	ld a,PW_ERR_NO_TABLE
	jp pw_leave

pwf_found:
	ld a,(pf_cur)
	call pw_range
	jp nz,pwf_corrupt
	ld (pf_ls),hl
	ld (pf_n),a
	ld a,(pf_cur)
	ld l,a
	ld h,0
	add hl,hl
	ld (pv_t2),hl
	ld de,(pv_blk)
	ld (de),a
	inc de
	PW_OUT2(PW_BY)
	ld a,(pf_cur)
	push de
	call pw_sizeptr
	pop de
	ldi
	ldi
	ldi
	ld a,(pf_depth)
	ld (de),a
	inc de
	ld hl,pf_parent
	ldi
	ldi
	ldi
	ld a,(pf_idx)
	ld (de),a
	inc de
	ld a,(pf_n)
	ld (de),a
	inc de
	PW_OUT2(PW_STEP)
	PW_OUT2(PW_BEGIN)
	ld a,(pf_n)
	or a
	jr z,pwf_done
	ld hl,(pf_ls)
	ld bc,PW_LINKS
	add hl,bc
	ld a,(pf_n)
	ld c,a
	ld b,0
	ldir
pwf_done:
	xor a
	jp pw_leave
