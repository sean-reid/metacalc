; The check job: verifies the unchecked assertions of a library below an end N one at a time with
; the kernel and writes each one's mark into the status table (store.inc). It is a module of the
; entry page. It never draws and never waits for a key: the caller owns the loop, the redraws and
; their pacing, and calls check_step as often as it likes between its own work.
;
; The including file has equates.inc, ram_map.inc, scr/ram.inc, check_ram.inc and store.inc, and
; provides the far entries far_kernel_reset, far_kernel_init, far_kernel_crc_begin,
; far_kernel_crc_chunk, far_kernel_verify and far_kernel_refs (far_kernel.inc), and far_lib_read and
; far_lib_assertion_count (far_reader.inc). The reader is open on the library (lib_open returned 0).
; IY is the OS flags pointer. Every routine destroys every register but IY, and the stack it uses
; is the caller's. The job's variables are in SCR_VARS (check_ram.inc) and survive a key wait, an
; auto power down and any number of OS calls; check_begin zeroes them, so the caller need not.
; The reference list is read through FM_SCRATCH, which the job holds only inside a call.
;
; Interrupts: the job never executes DI or EI. The kernel turns them off for each of its calls and
; gives the caller's state back, so a step enters and leaves as it was called. A tap on ON between two
; steps with interrupts off is reported by the next kernel call as status 14; with interrupts on the OS
; takes it. Speed: each kernel call runs at 15 MHz and the caller's speed is back before the job calls
; the store or returns, so the store, which may call the OS, runs at the caller's speed.
;
; Statuses, in A. B holds a detail where it is named.
; CK_READY       0  ready to step; with begin, a table exists and its counters are set
; CK_MORE        1  step: more may follow; crc_step: the run goes on
; CK_NEED_CRC    2  begin: no table for this library, the CRC run must pass first
; CK_DONE        3  step: no unchecked assertion below N is left
; CK_PAUSED      4  ON stopped the call; nothing was written for the assertion in hand
; CK_REFUSED     5  B = the kernel's status: the library, or the run's CRC, is refused
; CK_STORE       6  B = the store's status: the table is gone, archived or damaged
; CK_RANGE       7  begin: N is above the library's assertion count
; CK_READER      8  B = the reader's status
; CK_BAD_STATE   9  the routine does not fit the job's state
;
; check_begin     in:  A = the library's flash page, DE = the end N, at most the library's count
;                 out: A = status. Zeroes all of the job's variables first, so on every return,
;                      whatever the status, the counters, the first-failure record and the chunk
;                      count are 0 unless the table was counted (CK_READY). Resets and initialises
;                      the kernel, then reads the count and the CRC-32 from the library's header.
;                      With CK_RANGE, N is above the count. With CK_REFUSED, B = the kernel's status; kernel_init stopped
;                      by ON is CK_PAUSED. Then asks the store (store_has) for a table with this CRC
;                      and count: found, it is counted over the first N assertions and the job is
;                      CK_READY without a CRC run; not found, the job is CK_NEED_CRC and nothing is
;                      created. An archived table is not found. Costs: kernel_init, the header reads,
;                      one store_has and, with a table, one pass of store_count_to or store_count_ends
;                      (201,000 T-states on 3,326 assertions).
; check_crc_step  out: A = status. The first call after CK_NEED_CRC starts the kernel's CRC run
;                      (kernel_crc_begin, 307,000 T-states at most) and each later call reads one 16 KB
;                      chunk (2,027,000 at most): CK_MORE while the run goes on. The call that ends it
;                      with the header's CRC opens the table with store_open, which creates it, or
;                      replaces another library's, or unarchives this one's, and the job is CK_READY.
;                      A CRC that is wrong is CK_REFUSED with B = 1 and nothing is created. ON is
;                      CK_PAUSED and the next call starts the run again from its beginning. A store_open
;                      that fails is CK_STORE.
; check_step      out: A = status. Finds the next assertion below N whose value is 0 (store_next_zero),
;                      verifies it, and writes its mark: 2 for any refusal but ON, with the first such
;                      assertion's number and kernel status kept; for an accepted one, 1, or 3 when any
;                      assertion in its reference list has a stored value other than 1, dummy entries
;                      (bit 15) skipped. kernel_refs follows the verify at once; a stop reported by it
;                      is asked for again once, and a second stop pauses the step. The list is read from
;                      the library in pieces of CK_PIECE entries and each piece is tested against one
;                      lookup of the table, so a list of 511 entries is five reads. CK_MORE, or CK_DONE
;                      when the last zero below N is written, or when none is left. ON is CK_PAUSED with
;                      nothing written for that assertion; the next step verifies it again. A store
;                      failure, which is a table deleted, archived or damaged between steps, is CK_STORE
;                      with B = the store's status and stops the job: no table is created, the step's
;                      mark is not written, and every later call but check_begin and check_end is
;                      CK_BAD_STATE. A kernel status where the list should be, or a reader failure, is
;                      CK_REFUSED or CK_READER with B and stops it the same way. Costs besides the
;                      verify, measured on ring: a median of 19,200 T-states and 68,100 at worst, for
;                      79 references.
; check_end       out: A = 0, or the status of writing "next". Writes the number of the next assertion to
;                      check, the first unverified one of a paused job and N of a finished one, when the
;                      job holds a table, and calls kernel_reset whatever state the job is in, so no
;                      ready mark or reference list of the job stays in the kernel. The job is idle after
;                      it. "next" is only a hint: check_begin trusts the table, counting the entries
;                      below "next" in the same pass that counts the first N, and starts at "next" only
;                      when none of them is 0, else at the first 0 of the table.
; check_counts    out: HL = four words in RAM, the number of assertions of value 0, 1, 2 and 3 among the
;                      first N. Done is N - n0, failed is n2, resting is n3.
; check_first     out: A = 1 with DE = the number and B = the kernel's status of the first assertion this
;                      job found failing, or A = 0. Kept in RAM, never stored.
; check_chunks    out: A = the chunks the CRC run read before the call that ended it.
;
; Between steps the job detects a table that is deleted, archived or damaged (size, magic or
; version), and does not detect one replaced by another library's table of at least N entries: the
; store checks that a table is whole, not whose it is, and telling would take a lookup of the CRC in
; every step. It cannot happen while the app runs, since only the caller's own code and the OS's
; key wait run between steps, and neither replaces the variable.
;
; A mark of 3 can go stale if an assertion it rests on is checked again later; nothing repairs it.


CK_PORT_SPEED = 20h
CK_PIECE      = 120
CK_BUF        = FM_SCRATCH
CK_LIST       = FM_SCRATCH
CK_RESULTS    = FM_SCRATCH + 16

CK_READY      = 0
CK_MORE       = 1
CK_NEED_CRC   = 2
CK_DONE       = 3
CK_PAUSED     = 4
CK_REFUSED    = 5
CK_STORE      = 6
CK_RANGE      = 7
CK_READER     = 8
CK_BAD_STATE  = 9

CK_S_IDLE      = 0
CK_S_CRC_BEGIN = 1
CK_S_CRC_RUN   = 2
CK_S_READY     = 3
CK_S_STOPPED   = 4

check_begin:
	push de
	ld hl, ck_params
	ld d, h
	ld e, l
	inc de
	ld (hl), 0
	ld bc, CK_USED - 1
	ldir
	pop de
	ld (ck_params), a
	ld (ck_n), de
	ld hl, ck_params + 1
	ld (hl), 0FFh
	ld d, h
	ld e, l
	inc de
	ld bc, 7
	ldir
	call ck_fast
	call far_kernel_reset
	ld hl, ck_params
	call far_kernel_init
	ld b, a
	call ck_slow
	ld a, b
	or a
	jr z, cb_inited
	cp ST_INTERRUPTED
	ld a, CK_PAUSED
	ret z
	ld a, CK_REFUSED
	ret
cb_inited:
	call far_lib_assertion_count
	ld (ck_count), hl
	ld de, (ck_n)
	or a
	sbc hl, de
	jr nc, cb_in_range
	ld a, CK_RANGE
	ret
cb_in_range:
	ld bc, 4
	ld de, ck_crc
	ld hl, 4
	xor a
	call far_lib_read
	or a
	jr z, cb_have_crc
	ld b, a
	ld a, CK_READER
	ret
cb_have_crc:
	ld hl, (ck_crc)
	ld de, (ck_crc + 2)
	ld bc, (ck_count)
	call store_has
	or a
	jp z, ck_table
	ld a, CK_S_CRC_BEGIN
	ld (ck_state), a
	ld a, CK_NEED_CRC
	ret

check_crc_step:
	ld a, (ck_state)
	cp CK_S_CRC_BEGIN
	jr z, cc_begin
	cp CK_S_CRC_RUN
	ld a, CK_BAD_STATE
	ret nz
	call ck_fast
	call far_kernel_crc_chunk
	ld b, a
	call ck_slow
	ld a, b
	cp ST_CRC_MORE
	jr z, cc_more
	cp ST_INTERRUPTED
	jr z, cc_paused
	or a
	jr nz, cc_refused
	ld hl, (ck_crc)
	ld de, (ck_crc + 2)
	ld bc, (ck_count)
	call store_open
	or a
	jp nz, ck_stop
	jp ck_table
cc_more:
	ld hl, ck_chunks
	inc (hl)
	ld a, CK_MORE
	ret
cc_begin:
	call ck_fast
	call far_kernel_crc_begin
	ld b, a
	call ck_slow
	ld a, b
	or a
	jr z, cc_begun
	cp ST_INTERRUPTED
	jr nz, cc_refused
cc_paused:
	ld a, CK_S_CRC_BEGIN
	ld (ck_state), a
	xor a
	ld (ck_chunks), a
	ld a, CK_PAUSED
	ret
cc_begun:
	ld a, CK_S_CRC_RUN
	ld (ck_state), a
	ld a, CK_MORE
	ret
cc_refused:
	ld a, CK_S_STOPPED
	ld (ck_state), a
	ld a, CK_REFUSED
	ret

; In: A = a store status. Out: A = CK_STORE, B = the status, and the job stopped.
ck_stop:
	ld b, a
	ld a, CK_S_STOPPED
	ld (ck_state), a
	ld a, CK_STORE
	ret

; The job's table is there, by store_has or by store_open. Counts the first N assertions, and finds
; where to start. Out: A = CK_READY, or CK_STORE with B = the status.
ck_table:
	call store_get_next
	or a
	jr z, ct_hint
	cp ST_RANGE
	jp nz, ck_stop
	ld hl, 0
ct_hint:
	ld de, (ck_n)
	ld a, h
	or l
	jr z, ct_one
	push hl
	or a
	sbc hl, de
	pop hl
	jr nc, ct_one
	ld ix, CK_LIST
	ld (ix + 0), 2
	ld (ix + 1), l
	ld (ix + 2), h
	ld (ix + 3), e
	ld (ix + 4), d
	push hl
	ld hl, CK_LIST
	ld de, CK_RESULTS
	call store_count_ends
	pop de
	or a
	jp nz, ck_stop
	ld hl, (CK_RESULTS)
	ld a, h
	or l
	jr z, ct_hint_ok
	ld de, 0
ct_hint_ok:
	ld (ck_i), de
	ld hl, CK_RESULTS + 8
	ld de, ck_counts
	ld bc, 8
	ldir
	jr ct_ready
ct_one:
	ld hl, 0
	ld (ck_i), hl
	ld hl, ck_counts
	ld de, (ck_n)
	call store_count_to
	or a
	jp nz, ck_stop
ct_ready:
	ld a, CK_S_READY
	ld (ck_state), a
	xor a
	ret

check_end:
	ld a, (ck_state)
	cp CK_S_READY
	ld b, 0
	jr nz, ce_kernel
	ld hl, (ck_i)
	ld de, (ck_counts)
	ld a, d
	or e
	jr nz, ce_write
	ld hl, (ck_n)
ce_write:
	call store_set_next
	ld b, a
ce_kernel:
	push bc
	call ck_fast
	call far_kernel_reset
	call ck_slow
	xor a
	ld (ck_state), a
	pop bc
	ld a, b
	ret

check_counts:
	ld hl, ck_counts
	ret

check_first:
	ld a, (ck_have_first)
	or a
	ret z
	ld de, (ck_first)
	ld a, (ck_first_st)
	ld b, a
	ld a, 1
	ret

check_chunks:
	ld a, (ck_chunks)
	ret

; Sets 15 MHz, keeping the speed it found in ck_speed for ck_slow. Only the kernel calls run fast.
ck_fast:
	in a, (CK_PORT_SPEED)
	ld (ck_speed), a
	ld a, 1
	out (CK_PORT_SPEED), a
	ret

; Out: the speed ck_fast found. Destroys A.
ck_slow:
	ld a, (ck_speed)
	out (CK_PORT_SPEED), a
	ret

#include "check_step.inc"
