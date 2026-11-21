; Library cursor, header and symbol table.

magic:
	.db "MCL1"

; in: E:HL = library offset (below the library length)
; out: page mapped at $8000, HL = its address there, B = chunk index, C = flash page (the page of chunk 0 minus the index).
; Destroys AF.
map_offset:
	ld a,e
	add a,a
	add a,a
	ld b,a
	ld a,h
	rlca
	rlca
	and 3
	or b
	ld b,a
	ld a,(kv_lib_page)
	sub b
	ld c,a
	out (7),a
	ld a,h
	and $3F
	or $80
	ld h,a
	ret

; in: kv_rd_off. out: A = the byte there, kv_rd_off advanced. Refuses an offset at or past the length.
lib_u8:
	ld hl,kv_rd_off
	ld de,kv_length
	call cmp24
	jp nc,fail_malformed
	ld hl,(kv_rd_off)
	ld a,(kv_rd_off+2)
	ld e,a
	call map_offset
	ld a,(hl)
	ld hl,kv_rd_off
	inc (hl)
	ret nz
	inc hl
	inc (hl)
	ret nz
	inc hl
	inc (hl)
	ret

; in: A = byte count. out: kv_rd_off advanced; refused if that passes the length.
lib_skip:
	ld hl,(kv_rd_off)
	ld c,a
	ld b,0
	add hl,bc
	ld (kv_rd_off),hl
	jr nc,lib_skip_check
	ld hl,kv_rd_off+2
	inc (hl)
lib_skip_check:
	ld hl,kv_length
	ld de,kv_rd_off
	call cmp24
	jp c,fail_malformed
	ret

; in: HL = start, BC = count (not 0), A = value. Destroys AF BC DE HL.
fill_bytes:
	ld (hl),a
	dec bc
	ld e,l
	ld d,h
	inc de
	jp copy_bc

init_body:
	xor a
	ld (kv_listed),a
	ld a,h
	cp $C0
	jp nc,init_refused
	cp $BF
	jr c,init_param_ok
	ld a,l
	cp $101-PARAM_LEN
	jp nc,init_refused
init_param_ok:
	ld de,KERNEL_LIB_PAGE
	ld bc,PARAM_LEN
	ldir
	ld hl,0
	ld (kv_ready),hl
	ld (kv_crc_state),hl
	ld a,(KERNEL_LIB_PAGE)
	cp $80
	jp nc,fail_malformed
	ld (kv_lib_page),a
	out (7),a

	ld hl,$8000
	ld de,magic
	ld b,4
init_magic:
	ld a,(de)
	cp (hl)
	jp nz,fail_malformed
	inc de
	inc hl
	djnz init_magic

; length: below 2^24, at least the header, within the pages left in flash
	ld a,($800B)
	or a
	jp nz,fail_malformed
	ld hl,($8008)
	ld (kv_length),hl
	ld a,($800A)
	ld (kv_length+2),a
	ld a,(kv_lib_page)
	inc a
	ld b,a
	rrca
	rrca
	and $3F
	ld (kv_tmp24+2),a
	ld a,b
	and 3
	rrca
	rrca
	ld (kv_tmp24+1),a
	xor a
	ld (kv_tmp24),a
	ld hl,kv_tmp24
	ld de,kv_length
	call cmp24
	jp c,fail_malformed
	ld hl,kv_length
	ld de,header_len
	call cmp24
	jp c,fail_malformed

	ld a,($800C)
	ld (kv_constants),a
	ld c,a
	ld a,($800D)
	add a,c
	jp c,fail_malformed
	ld (kv_symbols),a
	ld hl,($800E)
	ld (kv_assertions),hl
	ld a,($8013)
	or a
	jp nz,fail_malformed
	ld hl,($8010)
	ld (kv_directory),hl
	ld a,($8012)
	ld (kv_directory+2),a

; chunk of the length and the readable tail inside it
	ld a,(kv_length+1)
	rlca
	rlca
	and 3
	ld c,a
	ld a,(kv_length+2)
	add a,a
	add a,a
	or c
	ld (kv_len_chunk),a
	ld a,(kv_length+1)
	and $3F
	or $80
	ld h,a
	ld a,(kv_length)
	ld l,a
	ld (kv_len_tail),hl

; typecode table: every id starts as a constant
	ld hl,TYPECODES
	ld bc,256
	ld a,$FF
	call fill_bytes
	ld hl,20
	ld (kv_rd_off),hl
	xor a
	ld (kv_rd_off+2),a
	ld (kv_sym_id),a
init_symbols:
	ld a,(kv_sym_id)
	ld hl,kv_symbols
	cp (hl)
	jr nc,init_symbols_done
	call lib_u8
	ld (kv_tmp8),a
	call lib_u8
	call lib_skip
	ld a,(kv_sym_id)
	ld hl,kv_constants
	cp (hl)
	jr nc,init_variable
	ld a,(kv_tmp8)
	cp $FF
	jp nz,fail_malformed
	jr init_symbol_ok
init_variable:
	ld a,(kv_tmp8)
	cp (hl)
	jp nc,fail_malformed
	ld a,(kv_sym_id)
	ld l,a
	ld h,TYPECODES>>8
	ld a,(kv_tmp8)
	ld (hl),a
init_symbol_ok:
	ld hl,kv_sym_id
	inc (hl)
	jr init_symbols
init_symbols_done:

; the symbol table ends inside the first chunk
	ld a,(kv_rd_off+2)
	or a
	jp nz,fail_malformed
	ld hl,(kv_rd_off)
	ld de,$4001
	or a
	sbc hl,de
	jp nc,fail_malformed

; directory: after the symbols, aligned, and every entry inside the library
	ld hl,kv_directory
	ld de,kv_rd_off
	call cmp24
	jp c,fail_malformed
	ld a,(kv_directory)
	and 3
	jp nz,fail_malformed
	ld hl,kv_length
	ld de,kv_directory
	call cmp24
	jp c,fail_malformed
	ld hl,(kv_assertions)
	ld e,0
	add hl,hl
	rl e
	add hl,hl
	rl e
	ld bc,(kv_directory)
	add hl,bc
	ld a,(kv_directory+2)
	adc a,e
	ld (kv_tmp24+2),a
	ld (kv_tmp24),hl
	ld hl,kv_length
	ld de,kv_tmp24
	call cmp24
	jp c,fail_malformed

; names, in the first chunk: 1 or more bytes in $21 to $7E but $24, all different
	ld a,(kv_symbols)
	or a
	jr z,init_names_done
	ld b,a
	ld a,(kv_lib_page)
	out (7),a
	ld hl,$8014
init_name_start:
	inc hl
	ld c,(hl)
	inc hl
	ld a,c
	or a
	jp z,fail_bad_symbol
init_name_byte:
	ld a,(hl)
	inc hl
	cp $21
	jp c,fail_bad_symbol
	cp $7F
	jp nc,fail_bad_symbol
	cp $24
	jp z,fail_bad_symbol
	dec c
	jr nz,init_name_byte
	djnz init_name_start
	ld a,(kv_symbols)
	dec a
	jr z,init_names_done
	ld (kv_tmp8),a
	ld hl,$8014
init_name_outer:
	in a,(4)
	and 8
	jp z,fail_interrupted
	inc hl
	ld c,(hl)
	inc hl
	ld d,h
	ld e,l
	ld a,c
	call add_hl_a
	ld a,(kv_tmp8)
	ld b,a
init_name_inner:
	inc hl
	ld a,(hl)
	inc hl
	cp c
	jr nz,init_name_skip
	push bc
	push de
	push hl
init_name_cmp:
	ld a,(de)
	cp (hl)
	jr nz,init_name_differ
	inc de
	inc hl
	dec c
	jr nz,init_name_cmp
	jp fail_bad_symbol
init_name_differ:
	pop hl
	pop de
	pop bc
	ld a,c
init_name_skip:
	call add_hl_a
	djnz init_name_inner
	ld h,d
	ld l,e
	ld a,c
	call add_hl_a
	ld a,(kv_tmp8)
	dec a
	ld (kv_tmp8),a
	jr nz,init_name_outer
init_names_done:

; limits, clamped to the buffers
	ld hl,(KERNEL_LIMIT_STACK_SYMS)
	ld de,CAP_STACK_SYMS
	call clamp_hl
	ld de,STACK_BUF
	add hl,de
	ld (kv_sym_limit_ptr),hl
	ld hl,(KERNEL_LIMIT_STACK_ENTRIES)
	ld de,CAP_STACK_ENTRIES
	call clamp_hl
	ld (kv_ent_limit),hl
	ld hl,(KERNEL_LIMIT_SAVED_SYMS)
	ld de,CAP_SAVED_SYMS
	call clamp_hl
	ld (kv_saved_sym_limit),hl
	ld hl,(KERNEL_LIMIT_SAVED_ENTRIES)
	ld de,CAP_SAVED_ENTRIES
	call clamp_hl
	ld (kv_saved_ent_limit),hl

; check cache for the ids it covers, substitution table
	ld hl,(kv_assertions)
	ld de,CACHE_IDS
	call clamp_hl
	ld de,7
	add hl,de
	srl h
	rr l
	srl h
	rr l
	srl h
	rr l
	ld b,h
	ld c,l
	ld a,b
	or c
	jr z,init_cache_clear
	ld hl,CACHE
	xor a
	call fill_bytes
init_cache_clear:
	call subst_clear
	ld hl,0
	ld (kv_serial),hl
	in a,(4)
	xor 8
	and 9
	jp nz,fail_interrupted
	ld hl,READY_MAGIC
	ld (kv_ready),hl
	ld c,0
	jp leave_c

init_refused:
	ld hl,0
	ld (kv_ready),hl
	ld (kv_crc_state),hl
	jp fail_malformed

; out: Z if the last kernel_init succeeded. Destroys AF DE HL.
is_ready:
	ld hl,(kv_ready)
	ld de,READY_MAGIC
	or a
	sbc hl,de
	ret

; Refuses unless the last kernel_init succeeded. Destroys AF DE HL.
require_ready:
	call is_ready
	ret z
	jp fail_malformed

; in: HL = value, DE = cap. out: HL = the smaller. Destroys AF DE.
clamp_hl:
	push hl
	or a
	sbc hl,de
	pop hl
	ret c
	ex de,hl
	ret

subst_clear:
	ld hl,SUBST
	ld bc,2048
	xor a
	jp fill_bytes

header_len:
	.db 20,0,0
