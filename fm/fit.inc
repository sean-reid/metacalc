; fm_fit: folds the statements of the loaded assertion until together they fit a number of lines,
; as the host model's fit and fit_one do (crates/mcui/src/fold.rs).
;
; The statements are numbered as the store holds them, the conclusion last. Each is selected in turn
; and its lines counted with fm_count, which keeps the count, so a fold changes the count of the one
; statement it touched and the total is a sum of bytes. fv_f0 to fv_f1 - 1 are the statements a pass
; works on, fv_ftgt the lines they must fit and fv_fs the one it is at.

; in: B = the lines to fit, D = the width of a hypothesis, E = the width of the conclusion
fit_body:
	call fm_need
	jp nz,fm_fail
	ld a,b
	ld (fv_frows),a
	ld a,d
	ld (fv_fhw),a
	ld a,e
	ld (fv_fcw),a
	call fm_same_library
	jp nz,fm_fail
	ld a,(fv_last)
	inc a
	ld a,FM_ERR_INDEX
	jp z,fm_fail
	xor a
	ld (fv_fs),a
ft_build:
	ld a,(fv_fs)
	call sel_index
	ld hl,ft_built
	ld (fv_then),hl
	jp nodes_have
ft_built:
	ld hl,fv_fs
	inc (hl)
	ld a,(fv_nw)
	cp (hl)
	jr nz,ft_build
		xor a
	ld (fv_fs),a
ft_init:
	call ft_reset
	ld hl,fv_fs
	inc (hl)
	ld a,(fv_nw)
	cp (hl)
	jr nz,ft_init
	ld (fv_f1),a
	xor a
	ld (fv_f0),a
	ld a,(fv_frows)
	ld (fv_ftgt),a
	ld hl,fv_nw
	cp (hl)
	jr c,ft_alone
	call ft_levels
	jr nc,ft_done
ft_alone:
	xor a
	ld (fv_fo),a
ft_one:
	ld a,(fv_fo)
	ld (fv_fs),a
	call ft_reset
	ld a,(fv_fo)
	ld (fv_f0),a
	inc a
	ld (fv_f1),a
	ld a,1
	ld (fv_ftgt),a
	call ft_levels
	ld hl,fv_fo
	inc (hl)
	ld a,(fv_nw)
	cp (hl)
	jr nz,ft_one
	ld hl,fv_nw
	ld a,(fv_frows)
	cp (hl)
	jr nc,ft_done
	ld a,(fv_last)
	ld (fv_fs),a
	ld (fv_f0),a
	inc a
	ld (fv_f1),a
	call ft_reset
	ld a,(fv_frows)
	ld (fv_ftgt),a
	call ft_levels
	jr nc,ft_done
	ld a,(fv_last)
	ld (fv_fs),a
	call ft_reset
	ld a,1
	ld (fv_ftgt),a
	call ft_levels
ft_done:
	xor a
	ld (fv_f0),a
	ld a,(fv_nw)
	ld (fv_f1),a
	call ft_sum
	push hl
	ld a,(fv_last)
	call sel_index
	ld a,$FF
	ld (fv_curn),a
	pop hl
	xor a
	ld b,a
	jp fm_leave

; Folds statements fv_f0 to fv_f1 - 1 level by level, deepest first, until their counted lines are
; at most fv_ftgt. out: NC when they fit, C when every level is folded and they do not.
ft_levels:
	call ft_fits
	ret nc
	call ft_deepest
fl_depth:
	ld a,(fv_f0)
	ld (fv_fs),a
fl_stmt:
	ld a,(fv_fs)
	call sel_index
	call ct_slot
	ld a,(hl)
	cp 2
	jr c,fl_next
	call ft_fold_depth
	or a
	jr z,fl_next
	call ft_dirty
	call ft_count
	call ft_fits
	ret nc
fl_next:
	ld hl,fv_fs
	inc (hl)
	ld a,(fv_f1)
	cp (hl)
	jr nz,fl_stmt
	ld a,(fv_fd)
	or a
	jr z,fl_none
	dec a
	ld (fv_fd),a
	jr fl_depth
fl_none:
	scf
	ret

; Folds every foldable node of depth fv_fd the selected statement has not folded. out: A = 1 when a
; flag changed, else 0. Destroys all but IX and IY.
ft_fold_depth:
	xor a
	ld (fv_fchg),a
	call fo_table
	ld b,a
	ld hl,(fv_tb)
	ld de,2*FM_PLANE
	add hl,de
	ld a,(fv_fd)
	ld c,a
fd_loop:
	ld a,(hl)
	cp c
	jr nz,fd_next
	ld a,b
	cp 1
	jr z,fd_next
	inc hl
	ld a,(hl)
	dec hl
	cp c
	jr z,fd_next
	jr c,fd_next
	push hl
	ld de,FM_PLANE
	add hl,de
	ld a,(hl)
	or a
	jr nz,fd_seen
	ld (hl),1
	ld a,1
	ld (fv_fchg),a
fd_seen:
	pop hl
fd_next:
	inc hl
	djnz fd_loop
	ld a,(fv_fchg)
	ret

; out: fv_fd = the deepest node of the tables of statements fv_f0 to fv_f1 - 1. Destroys all but
; IX and IY.
ft_deepest:
	xor a
	ld (fv_fd),a
	ld a,(fv_f0)
	ld (fv_fs),a
dp_stmt:
	ld a,(fv_fs)
	call sel_index
	call fo_table
	ld b,a
	ld hl,(fv_tb)
	ld de,2*FM_PLANE
	add hl,de
	ld a,(fv_fd)
	ld c,a
dp_loop:
	ld a,(hl)
	cp c
	jr c,dp_next
	ld c,a
dp_next:
	inc hl
	djnz dp_loop
	ld a,c
	ld (fv_fd),a
	ld hl,fv_fs
	inc (hl)
	ld a,(fv_f1)
	cp (hl)
	jr nz,dp_stmt
	ret

; Selects statement fv_fs, clears its folds and counts its lines at its width. Destroys all but IX
; and IY.
ft_reset:
	ld a,(fv_fs)
	call sel_index
	call fo_table
	ld b,a
	ld hl,(fv_tb)
	ld de,3*FM_PLANE
	add hl,de
	ld c,0
rs_loop:
	ld a,(hl)
	or a
	jr z,rs_next
	ld (hl),0
	ld c,1
rs_next:
	inc hl
	djnz rs_loop
	ld a,c
	or a
	call nz,ft_dirty

; out: A = the lines statement fv_fs, selected, takes at its width. Destroys all but IX and IY.
ft_count:
	ld a,(fv_fs)
	ld hl,fv_last
	cp (hl)
	ld a,(fv_fhw)
	jr nz,fc_set
	ld a,(fv_fcw)
fc_set:
	ld (fv_limit),a
	jp fm_count

; out: HL = the counted lines of statements fv_f0 to fv_f1 - 1 together. Destroys AF BC DE.
ft_sum:
	ld a,(fv_f0)
	ld e,a
	ld d,0
	ld hl,fv_rows
	add hl,de
	ld a,(fv_f1)
	sub e
	ld b,a
	ld de,0
fs_add:
	ld c,(hl)
	ld a,e
	add a,c
	ld e,a
	jr nc,fs_next
	inc d
fs_next:
	inc hl
	djnz fs_add
	ex de,hl
	ret

; out: C when the counted lines of statements fv_f0 to fv_f1 - 1 are more than fv_ftgt. Destroys AF
; BC DE HL.
ft_fits:
	call ft_sum
	ld a,h
	or a
	scf
	ret nz
	ld a,(fv_ftgt)
	cp l
	ret

; Marks the selected statement's records stale, which also drops its line count. Destroys AF DE HL.
ft_dirty:
	ld hl,(fv_hdr)
	ld de,7
	add hl,de
	ld (hl),1
	ret
