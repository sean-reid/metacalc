; Reading the records of the assertions a proof names, straight from the flash page that holds
; each, with the walk's own bounds checks. All of it runs inside the session of the walk: on the
; private stack, with the spare page in bank C and a library page in bank B.

; in: A:HL = a library offset. out: Z, port 7 on the page that holds it, HL = its address in bank B
; and DE = the address past the last byte of the library that page shows, at most $C000; or NZ and
; A = LIB_ERR_BAD when the offset is not inside the library. Destroys BC.
pw_map:
	ld b,a
	ex de,hl
	ld hl,(pv_len)
	or a
	sbc hl,de
	ld a,(pv_len+2)
	sbc a,b
	jr c,pwm_bad
	ld c,a
	or h
	or l
	jr z,pwm_bad
	push hl
	ld a,d
	rlca
	rlca
	and 3
	ld l,a
	ld a,b
	add a,a
	add a,a
	or l
	ld l,a
	ld a,(pv_lpage)
	sub l
	out (7),a
	ld a,d
	and $3F
	or $80
	ld d,a
	pop hl
	ld a,c
	or a
	jr nz,pwm_full
	add hl,de
	jr c,pwm_full
	ld a,h
	cp $C0
	jr nc,pwm_full
	jr pwm_limit
pwm_full:
	ld hl,$C000
pwm_limit:
	ex de,hl
	xor a
	ret
pwm_bad:
	ld a,LIB_ERR_BAD
	or a
	ret

; in: HL = an address, BC = a count. out: Z when HL + BC is not past pv_lim, else NZ and A =
; LIB_ERR_BAD. Keeps HL and BC.
pw_room:
	push hl
	add hl,bc
	jr c,pwo_no
	ld de,(pv_lim)
	ex de,hl
	or a
	sbc hl,de
	jr c,pwo_no
	pop hl
	xor a
	ret
pwo_no:
	pop hl
	ld a,LIB_ERR_BAD
	or a
	ret

; in: HL = the number of an assertion below the theorem's. out: Z, B = its hypothesis count and
; C = 1 when its conclusion's typecode is the provable one, else 0; or NZ and A = LIB_ERR_BAD for a
; directory entry or a record the format forbids or that leaves the library or the chunk. Leaves port
; 7 on the record's page. Destroys all but IX, IY.
pw_head:
	xor a
	add hl,hl
	rla
	add hl,hl
	rla
	ld bc,(pv_dir)
	add hl,bc
	ld c,a
	ld a,(pv_dir+2)
	adc a,c
	call pw_map
	ret nz
	ld (pv_lim),de
	ld bc,4
	call pw_room
	ret nz
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	ld b,(hl)
	inc hl
	ld a,(hl)
	or a
	jr nz,pwh_bad
	ex de,hl
	ld a,b
	call pw_map
	ret nz
	ld (pv_lim),de
	ld bc,2
	call pw_room
	ret nz
	inc hl
	ld c,(hl)
	inc hl
	ld b,0
	add hl,bc
	jr c,pwh_bad
	ld bc,1
	call pw_room
	ret nz
	ld a,(hl)
	inc hl
	ld (pv_nh),a
	or a
	jr z,pwh_pairs
	ld (pv_cnt),a
pwh_loop:
	ld bc,2
	call pw_room
	ret nz
	ld a,(hl)
	or a
	jr z,pwh_float
	cp 1
	jr nz,pwh_bad
	inc hl
	ld c,(hl)
	inc hl
	ld b,0
	add hl,bc
	jr c,pwh_bad
	jr pwh_next
pwh_float:
	inc hl
	inc hl
pwh_next:
	ld a,(pv_cnt)
	dec a
	ld (pv_cnt),a
	jr nz,pwh_loop
pwh_pairs:
	ld bc,2
	call pw_room
	ret nz
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	ex de,hl
	add hl,hl
	jr c,pwh_bad
	add hl,de
	jr c,pwh_bad
	ld bc,2
	call pw_room
	ret nz
	ld a,(hl)
	or a
	jr z,pwh_bad
	inc hl
	ld a,(hl)
	ld hl,pv_tc
	cp (hl)
	ld c,0
	jr nz,pwh_done
	inc c
pwh_done:
	ld a,(pv_nh)
	ld b,a
	xor a
	ret
pwh_bad:
	ld a,LIB_ERR_BAD
	or a
	ret
