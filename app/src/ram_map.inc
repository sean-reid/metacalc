; The RAM map: the regions of RAM the kernel, the kernel driver, the UI, the reader and the cross-page call use, in
; address order. The probes keep scratch outside it and check it against the regions of what they link.
; Each region is NAME for its base, NAME_SIZE for its size, and can be moved with -DNAME=$address.
; The file that owns a region lays out the bytes inside it. Assembling any image that includes
; this file fails when two regions overlap or a region leaves its bank.
;
; SCR_VARS and SCR_STACK hold state that must outlive a wait in _GetKey. The figures here are
; measured under OS 2.43 in the emulator; no calculator was tried. Both regions kept every byte
; through the events crates/mcemu/tests/screen_state.rs drives: keys, 2nd, ALPHA, alpha lock, the
; contrast keys, ON, an auto power down with ON to wake, an ONSCRPT that runs, the OS calls that
; test makes, and an app that leaves without touching them. The shell's exit, app_putaway, fills
; SCR_STACK with spaces and SCR_VARS with zeros on every way out. appData did not keep its
; bytes: with an ONSCRPT AppVar in RAM and bit 0 of the flag byte at iy+33h set, the OS copied a
; zero and the script's first 63 bytes over 8000h when it woke, so the state is not there; an
; OFFSCRPT in RAM with bit 1 set is copied the same way at 2nd+OFF. At an auto power down the OS
; copied the screen's pixels into saveSScreen.
;
;   name              bank  where
;   UI_VARS           B     the UI's variables, in saveSScreen; the OS overwrites it with the
;                           screen's pixels when auto power down fires in _GetKey
;   SCR_VARS          B     the framework's variables, 108 bytes: smallEditRAM, 90D3h to 913Eh,
;                           XFact follows
;   UI_FRAME          B     the UI's 96 by 64 frame, plotSScreen
;   SCR_STACK         B     the stack of open screens, 128 bytes: cmdShadow, 966Eh to 96EDh,
;                           cmdShadCur follows
;   HOME_STATE        B     the OS's home screen state that app_putaway writes, 2 bytes: cmdShadCur, 96EEh;
;                           no region may overlap it
;   HOME_PENDING      B     the OS's entry line pointers that app_putaway reads, 4 bytes: editTop, 96F4h, and
;                           editCursor; no region may overlap it, and it is not declared as a region
;                           the tests mask
;   KD_SCRATCH        B     the kernel driver's scratch, from libPage, in appBackUpScreen. The app's own
;                           two bytes are libPage (offset 0) and appStatus (offset 1), checked in
;                           equates.inc; probe/kerndriver.inc lays out its own variables over the same
;                           bytes in its own image, where offset 1 is not appStatus
;   FAR_VARS          B     the cross-page call's scratch (app/src/farcall.inc), in the gap
;                           between KD_SCRATCH and LIB_STATE
;   LIB_STATE         B     the reader's state, after the driver's scratch
;   KERNEL_RAM_LOW    C     the kernel's low region
;   KERNEL_RAM_HIGH   C     the kernel's high region
;   LIB_BOUNCE        C     an overlay on the last 256 bytes of the kernel's high region
;   FM_SCRATCH        B     fm's 256 byte buffer for the reader's accessors, in saveSScreen after
;                           UI_VARS; it holds nothing across calls
;   APP_BUF           B     the shell's 256 byte buffer for what the top screen's enter or draw builds
;                           again, in saveSScreen after FM_SCRATCH, since the OS writes the screen's
;                           pixels into saveSScreen at an auto power down. Its variables follow the
;                           regions, from APP_COUNTS to APP_FRAME, each placed by APP_VAR
;   APP_STAGE         B     the shell's staging area for constant strings, 128 bytes in saveSScreen
;                           after APP_BUF: the canvas reads a string with its own page mapped, so the
;                           entry page copies the strings of the screen it draws here first
;                           (app_stage_block, text.inc). Offset 0 is the tag, the two byte address of the
;                           block it holds, 0 for none; the block starts at offset 2, at most 126
;                           bytes. A screen's block is the constant strings it draws, laid out by
;                           strings.inc, and is the only thing the area holds, so a copy cannot
;                           touch APP_BUF, where the strings built at run time are. The OS writes the
;                           screen's pixels over it at an auto power down: the shell clears the tag
;                           before the refresh that follows, and the next draw copies the block again
;   APP_SENTINEL      B     the shell's one byte, in saveSScreen after APP_STAGE, the last byte of it: the
;                           complement of the byte of UI_FRAME at the same offset, stored after every
;                           blit, so that a pixel copy over saveSScreen is told from the screen
;                           being intact (metacalc.z80)
;   FM_RAM            C     fm's working memory, an overlay on the kernel's high region from its
;                           start; the kernel uses that region only inside a kernel call, so fm
;                           and the kernel share it through the ready mark (fm/fm.z80)
;   FM_LOW            C     fm's second working region, an overlay on the kernel's low region from
;                           KERNEL_RAM_LOW + $100 to its end, shared with the kernel the same way;
;                           the page below it holds the kernel's parameters and ready mark and fm
;                           never writes it
#ifndef RAM_MAP
#define RAM_MAP

#ifndef UI_VARS
UI_VARS             .equ $86EC
#endif
#ifndef UI_VARS_SIZE
UI_VARS_SIZE        .equ $1A
#endif
#ifndef SCR_VARS
SCR_VARS            .equ $90D3
#endif
#ifndef SCR_VARS_SIZE
SCR_VARS_SIZE       .equ 108
#endif
#ifndef SCR_STACK
SCR_STACK           .equ $966E
#endif
#ifndef SCR_STACK_SIZE
SCR_STACK_SIZE      .equ 128
#endif
#ifndef HOME_STATE
HOME_STATE          .equ $96EE
#endif
#ifndef HOME_PENDING
HOME_PENDING        .equ $96F4
#endif
#ifndef HOME_PENDING_LEN
HOME_PENDING_LEN    .equ 4
#endif
#ifndef HOME_STATE_SIZE
HOME_STATE_SIZE     .equ 2
#endif
#ifndef UI_FRAME
UI_FRAME            .equ $9340
#endif
#ifndef UI_FRAME_SIZE
UI_FRAME_SIZE       .equ 768
#endif
#ifndef KD_SCRATCH
KD_SCRATCH          .equ $9872
#endif
#ifndef KD_SCRATCH_SIZE
KD_SCRATCH_SIZE     .equ 36
#endif
#ifndef FAR_VARS
FAR_VARS            .equ $9896
#endif
#ifndef FAR_VARS_SIZE
FAR_VARS_SIZE       .equ 4
#endif
#ifndef LIB_STATE
LIB_STATE           .equ $989C
#endif
#ifndef LIB_STATE_SIZE
LIB_STATE_SIZE      .equ $2D5
#endif
#ifndef KERNEL_RAM_LOW
KERNEL_RAM_LOW      .equ $C500
#endif
LOW_SIZE            .equ $1200
#ifndef KERNEL_RAM_HIGH
KERNEL_RAM_HIGH     .equ $DF00
#endif
HIGH_SIZE           .equ $2000
LIB_BOUNCE_SIZE     .equ $100
#ifndef LIB_BOUNCE
LIB_BOUNCE          .equ KERNEL_RAM_HIGH + HIGH_SIZE - LIB_BOUNCE_SIZE
#endif

#ifndef FM_SCRATCH
FM_SCRATCH          .equ UI_VARS + UI_VARS_SIZE
#endif
FM_SCRATCH_SIZE     .equ $100
#ifndef APP_BUF
APP_BUF             .equ UI_VARS + UI_VARS_SIZE + FM_SCRATCH_SIZE
#endif
APP_BUF_SIZE        .equ $100
#ifndef APP_STAGE
APP_STAGE           .equ $8906
#endif
#ifndef APP_STAGE_SIZE
APP_STAGE_SIZE      .equ $80
#endif
APP_STAGE_TAG       .equ APP_STAGE
APP_STAGE_DATA      .equ APP_STAGE + 2
APP_STAGE_DATA_SIZE .equ APP_STAGE_SIZE - 2
#ifndef APP_SENTINEL
APP_SENTINEL        .equ $89EB
#endif
APP_SENTINEL_SIZE   .equ 1
#ifndef FM_RAM
FM_RAM              .equ KERNEL_RAM_HIGH
#endif
#ifndef FM_RAM_SIZE
FM_RAM_SIZE         .equ $1E00
#endif

#ifndef FM_LOW
FM_LOW              .equ KERNEL_RAM_LOW + $100
#endif
#ifndef FM_LOW_SIZE
FM_LOW_SIZE         .equ LOW_SIZE - $100
#endif

#macro RAM_DISJOINT(a_base, a_size, b_base, b_size, message)
#if (a_base < (b_base + b_size)) && (b_base < (a_base + a_size))
 .error message
#endif
#endmacro

#macro RAM_VAR(name, base, offset, size, limit, next, message)
name .equ base + offset
#if ((offset + size) > limit) || (offset < next) || (offset > next)
 .error message
#endif
#endmacro

#macro RAM_AT(offset, limit, next, message)
#if (offset > limit) || (offset < next)
 .error message
#endif
#endmacro

#macro RAM_WITHIN(base, size, low, high, message)
#if (base < low) || ((base + size) > high)
 .error message
#endif
#endmacro

; The variables of APP_BUF, in address order. Each APP_VAR checks that its variable neither overlaps
; the one before it nor runs past APP_BUF_SIZE.
;   APP_COUNTS  the check counts of the stages (screen_counts.inc): eight bytes for each of at most
;               six stages, the four little-endian words n0 to n3 of store_count_ends
;   APP_LIST    the list store_count_ends takes: the number of stages, then each stage's end
;   APP_CRC     the library's CRC-32 as the header holds it
;   APP_TEXT    what the top screen draws that is not a constant: the strings it builds and its
;               own variables, laid out by that screen's file
;   APP_HELD    the shell's one flag byte: not 0 while the key it handled may still be down
;               (metacalc.z80)
;   APP_FRAME   the frame the error notice is pushed from, outside SCR_STACK
#define APP_NEXT 0
#macro APP_VAR(name, offset, size)
RAM_VAR(name, APP_BUF, offset, size, APP_BUF_SIZE, APP_NEXT, "the APP_BUF variables overlap, leave a gap, or run past APP_BUF_SIZE in ram_map.inc")
#define APP_NEXT eval(offset + size)
#endmacro

APP_VAR(APP_COUNTS, $00, 48)
APP_VAR(APP_LIST, $30, 13)
APP_VAR(APP_CRC, $3D, 4)
APP_VAR(APP_TEXT, $41, 174)
APP_VAR(APP_HELD, $EF, 1)
APP_VAR(APP_FRAME, $F0, 16)
APP_COUNTS_SIZE     .equ APP_LIST - APP_COUNTS
APP_TEXT_SIZE       .equ APP_HELD - APP_TEXT

RAM_WITHIN(SCR_VARS, SCR_VARS_SIZE, $8000, $C000, "SCR_VARS runs past bank B")
RAM_WITHIN(SCR_STACK, SCR_STACK_SIZE, $8000, $C000, "SCR_STACK runs past bank B")
RAM_WITHIN(HOME_STATE, HOME_STATE_SIZE, $8000, $C000, "HOME_STATE runs past bank B")
RAM_WITHIN(HOME_PENDING, HOME_PENDING_LEN, $8000, $C000, "HOME_PENDING runs past bank B")
RAM_WITHIN(UI_VARS, UI_VARS_SIZE, $8000, $C000, "UI_VARS runs past bank B")
RAM_WITHIN(UI_FRAME, UI_FRAME_SIZE, $8000, $C000, "UI_FRAME runs past bank B")
RAM_WITHIN(KD_SCRATCH, KD_SCRATCH_SIZE, $8000, $C000, "KD_SCRATCH runs past bank B")
RAM_WITHIN(FAR_VARS, FAR_VARS_SIZE, $8000, $C000, "FAR_VARS runs past bank B")
RAM_WITHIN(LIB_STATE, LIB_STATE_SIZE, $8000, $C000, "LIB_STATE runs past bank B")
RAM_WITHIN(KERNEL_RAM_LOW, LOW_SIZE, $C000, $10000, "KERNEL_RAM_LOW runs past bank C")
RAM_WITHIN(KERNEL_RAM_HIGH, HIGH_SIZE, $C000, $10000, "KERNEL_RAM_HIGH runs past bank C")

RAM_DISJOINT(UI_VARS, UI_VARS_SIZE, UI_FRAME, UI_FRAME_SIZE, "UI_FRAME and UI_VARS overlap")
RAM_DISJOINT(UI_VARS, UI_VARS_SIZE, KD_SCRATCH, KD_SCRATCH_SIZE, "UI_VARS and KD_SCRATCH overlap")
RAM_DISJOINT(UI_VARS, UI_VARS_SIZE, LIB_STATE, LIB_STATE_SIZE, "UI_VARS and LIB_STATE overlap")
RAM_DISJOINT(UI_FRAME, UI_FRAME_SIZE, KD_SCRATCH, KD_SCRATCH_SIZE, "UI_FRAME and KD_SCRATCH overlap")
RAM_DISJOINT(UI_FRAME, UI_FRAME_SIZE, LIB_STATE, LIB_STATE_SIZE, "UI_FRAME and LIB_STATE overlap")
RAM_DISJOINT(KD_SCRATCH, KD_SCRATCH_SIZE, LIB_STATE, LIB_STATE_SIZE, "KD_SCRATCH and LIB_STATE overlap")
RAM_DISJOINT(FAR_VARS, FAR_VARS_SIZE, UI_VARS, UI_VARS_SIZE, "FAR_VARS and UI_VARS overlap")
RAM_DISJOINT(FAR_VARS, FAR_VARS_SIZE, UI_FRAME, UI_FRAME_SIZE, "FAR_VARS and UI_FRAME overlap")
RAM_DISJOINT(FAR_VARS, FAR_VARS_SIZE, KD_SCRATCH, KD_SCRATCH_SIZE, "FAR_VARS and KD_SCRATCH overlap")
RAM_DISJOINT(FAR_VARS, FAR_VARS_SIZE, LIB_STATE, LIB_STATE_SIZE, "FAR_VARS and LIB_STATE overlap")
RAM_DISJOINT(SCR_VARS, SCR_VARS_SIZE, SCR_STACK, SCR_STACK_SIZE, "SCR_VARS and SCR_STACK overlap")
RAM_DISJOINT(SCR_VARS, SCR_VARS_SIZE, UI_VARS, UI_VARS_SIZE, "SCR_VARS and UI_VARS overlap")
RAM_DISJOINT(SCR_VARS, SCR_VARS_SIZE, UI_FRAME, UI_FRAME_SIZE, "SCR_VARS and UI_FRAME overlap")
RAM_DISJOINT(SCR_VARS, SCR_VARS_SIZE, KD_SCRATCH, KD_SCRATCH_SIZE, "SCR_VARS and KD_SCRATCH overlap")
RAM_DISJOINT(SCR_VARS, SCR_VARS_SIZE, FAR_VARS, FAR_VARS_SIZE, "SCR_VARS and FAR_VARS overlap")
RAM_DISJOINT(SCR_VARS, SCR_VARS_SIZE, LIB_STATE, LIB_STATE_SIZE, "SCR_VARS and LIB_STATE overlap")
RAM_DISJOINT(SCR_STACK, SCR_STACK_SIZE, UI_VARS, UI_VARS_SIZE, "SCR_STACK and UI_VARS overlap")
RAM_DISJOINT(SCR_STACK, SCR_STACK_SIZE, UI_FRAME, UI_FRAME_SIZE, "SCR_STACK and UI_FRAME overlap")
RAM_DISJOINT(SCR_STACK, SCR_STACK_SIZE, KD_SCRATCH, KD_SCRATCH_SIZE, "SCR_STACK and KD_SCRATCH overlap")
RAM_DISJOINT(SCR_STACK, SCR_STACK_SIZE, FAR_VARS, FAR_VARS_SIZE, "SCR_STACK and FAR_VARS overlap")
RAM_DISJOINT(SCR_STACK, SCR_STACK_SIZE, LIB_STATE, LIB_STATE_SIZE, "SCR_STACK and LIB_STATE overlap")
RAM_DISJOINT(KERNEL_RAM_LOW, LOW_SIZE, KERNEL_RAM_HIGH, HIGH_SIZE, "KERNEL_RAM_LOW and KERNEL_RAM_HIGH overlap")

RAM_WITHIN(FM_SCRATCH, FM_SCRATCH_SIZE, $8000, $C000, "FM_SCRATCH runs past bank B")
RAM_WITHIN(APP_BUF, APP_BUF_SIZE, $8000, $C000, "APP_BUF runs past bank B")
RAM_WITHIN(APP_STAGE, APP_STAGE_SIZE, $8000, $C000, "APP_STAGE runs past bank B")
RAM_WITHIN(APP_SENTINEL, APP_SENTINEL_SIZE, $8000, $C000, "APP_SENTINEL runs past bank B")
RAM_WITHIN(FM_RAM, FM_RAM_SIZE, KERNEL_RAM_HIGH, KERNEL_RAM_HIGH + HIGH_SIZE, "FM_RAM runs outside the kernel's high region")
#if FM_LOW < (KERNEL_RAM_LOW + $100)
 .error "FM_LOW must not start below KERNEL_RAM_LOW + $100, the page of the kernel's parameters"
#endif
RAM_WITHIN(FM_LOW, FM_LOW_SIZE, KERNEL_RAM_LOW, KERNEL_RAM_LOW + LOW_SIZE, "FM_LOW runs outside the kernel's low region")
RAM_DISJOINT(FM_SCRATCH, FM_SCRATCH_SIZE, SCR_VARS, SCR_VARS_SIZE, "FM_SCRATCH and SCR_VARS overlap")
RAM_DISJOINT(FM_SCRATCH, FM_SCRATCH_SIZE, SCR_STACK, SCR_STACK_SIZE, "FM_SCRATCH and SCR_STACK overlap")
RAM_DISJOINT(FM_SCRATCH, FM_SCRATCH_SIZE, UI_VARS, UI_VARS_SIZE, "FM_SCRATCH and UI_VARS overlap")
RAM_DISJOINT(FM_SCRATCH, FM_SCRATCH_SIZE, UI_FRAME, UI_FRAME_SIZE, "FM_SCRATCH and UI_FRAME overlap")
RAM_DISJOINT(FM_SCRATCH, FM_SCRATCH_SIZE, KD_SCRATCH, KD_SCRATCH_SIZE, "FM_SCRATCH and KD_SCRATCH overlap")
RAM_DISJOINT(FM_SCRATCH, FM_SCRATCH_SIZE, FAR_VARS, FAR_VARS_SIZE, "FM_SCRATCH and FAR_VARS overlap")
RAM_DISJOINT(FM_SCRATCH, FM_SCRATCH_SIZE, LIB_STATE, LIB_STATE_SIZE, "FM_SCRATCH and LIB_STATE overlap")
RAM_DISJOINT(APP_BUF, APP_BUF_SIZE, UI_VARS, UI_VARS_SIZE, "APP_BUF and UI_VARS overlap")
RAM_DISJOINT(APP_BUF, APP_BUF_SIZE, UI_FRAME, UI_FRAME_SIZE, "APP_BUF and UI_FRAME overlap")
RAM_DISJOINT(APP_BUF, APP_BUF_SIZE, SCR_VARS, SCR_VARS_SIZE, "APP_BUF and SCR_VARS overlap")
RAM_DISJOINT(APP_BUF, APP_BUF_SIZE, SCR_STACK, SCR_STACK_SIZE, "APP_BUF and SCR_STACK overlap")
RAM_DISJOINT(APP_BUF, APP_BUF_SIZE, KD_SCRATCH, KD_SCRATCH_SIZE, "APP_BUF and KD_SCRATCH overlap")
RAM_DISJOINT(APP_BUF, APP_BUF_SIZE, FAR_VARS, FAR_VARS_SIZE, "APP_BUF and FAR_VARS overlap")
RAM_DISJOINT(APP_BUF, APP_BUF_SIZE, LIB_STATE, LIB_STATE_SIZE, "APP_BUF and LIB_STATE overlap")
RAM_DISJOINT(APP_BUF, APP_BUF_SIZE, FM_SCRATCH, FM_SCRATCH_SIZE, "APP_BUF and FM_SCRATCH overlap")
RAM_DISJOINT(HOME_STATE, HOME_STATE_SIZE, UI_VARS, UI_VARS_SIZE, "HOME_STATE and UI_VARS overlap")
RAM_DISJOINT(HOME_STATE, HOME_STATE_SIZE, UI_FRAME, UI_FRAME_SIZE, "HOME_STATE and UI_FRAME overlap")
RAM_DISJOINT(HOME_STATE, HOME_STATE_SIZE, KD_SCRATCH, KD_SCRATCH_SIZE, "HOME_STATE and KD_SCRATCH overlap")
RAM_DISJOINT(HOME_STATE, HOME_STATE_SIZE, FAR_VARS, FAR_VARS_SIZE, "HOME_STATE and FAR_VARS overlap")
RAM_DISJOINT(HOME_STATE, HOME_STATE_SIZE, LIB_STATE, LIB_STATE_SIZE, "HOME_STATE and LIB_STATE overlap")
RAM_DISJOINT(HOME_STATE, HOME_STATE_SIZE, SCR_VARS, SCR_VARS_SIZE, "HOME_STATE and SCR_VARS overlap")
RAM_DISJOINT(HOME_STATE, HOME_STATE_SIZE, SCR_STACK, SCR_STACK_SIZE, "HOME_STATE and SCR_STACK overlap")
RAM_DISJOINT(HOME_STATE, HOME_STATE_SIZE, FM_SCRATCH, FM_SCRATCH_SIZE, "HOME_STATE and FM_SCRATCH overlap")
RAM_DISJOINT(HOME_STATE, HOME_STATE_SIZE, APP_BUF, APP_BUF_SIZE, "HOME_STATE and APP_BUF overlap")
RAM_DISJOINT(HOME_PENDING, HOME_PENDING_LEN, UI_VARS, UI_VARS_SIZE, "HOME_PENDING and UI_VARS overlap")
RAM_DISJOINT(HOME_PENDING, HOME_PENDING_LEN, UI_FRAME, UI_FRAME_SIZE, "HOME_PENDING and UI_FRAME overlap")
RAM_DISJOINT(HOME_PENDING, HOME_PENDING_LEN, KD_SCRATCH, KD_SCRATCH_SIZE, "HOME_PENDING and KD_SCRATCH overlap")
RAM_DISJOINT(HOME_PENDING, HOME_PENDING_LEN, FAR_VARS, FAR_VARS_SIZE, "HOME_PENDING and FAR_VARS overlap")
RAM_DISJOINT(HOME_PENDING, HOME_PENDING_LEN, LIB_STATE, LIB_STATE_SIZE, "HOME_PENDING and LIB_STATE overlap")
RAM_DISJOINT(HOME_PENDING, HOME_PENDING_LEN, SCR_VARS, SCR_VARS_SIZE, "HOME_PENDING and SCR_VARS overlap")
RAM_DISJOINT(HOME_PENDING, HOME_PENDING_LEN, SCR_STACK, SCR_STACK_SIZE, "HOME_PENDING and SCR_STACK overlap")
RAM_DISJOINT(HOME_PENDING, HOME_PENDING_LEN, FM_SCRATCH, FM_SCRATCH_SIZE, "HOME_PENDING and FM_SCRATCH overlap")
RAM_DISJOINT(HOME_PENDING, HOME_PENDING_LEN, APP_BUF, APP_BUF_SIZE, "HOME_PENDING and APP_BUF overlap")
RAM_DISJOINT(HOME_STATE, HOME_STATE_SIZE, HOME_PENDING, HOME_PENDING_LEN, "HOME_STATE and HOME_PENDING overlap")
RAM_DISJOINT(APP_SENTINEL, APP_SENTINEL_SIZE, UI_VARS, UI_VARS_SIZE, "APP_SENTINEL and UI_VARS overlap")
RAM_DISJOINT(APP_SENTINEL, APP_SENTINEL_SIZE, UI_FRAME, UI_FRAME_SIZE, "APP_SENTINEL and UI_FRAME overlap")
RAM_DISJOINT(APP_SENTINEL, APP_SENTINEL_SIZE, SCR_VARS, SCR_VARS_SIZE, "APP_SENTINEL and SCR_VARS overlap")
RAM_DISJOINT(APP_SENTINEL, APP_SENTINEL_SIZE, SCR_STACK, SCR_STACK_SIZE, "APP_SENTINEL and SCR_STACK overlap")
RAM_DISJOINT(APP_SENTINEL, APP_SENTINEL_SIZE, KD_SCRATCH, KD_SCRATCH_SIZE, "APP_SENTINEL and KD_SCRATCH overlap")
RAM_DISJOINT(APP_SENTINEL, APP_SENTINEL_SIZE, FAR_VARS, FAR_VARS_SIZE, "APP_SENTINEL and FAR_VARS overlap")
RAM_DISJOINT(APP_SENTINEL, APP_SENTINEL_SIZE, LIB_STATE, LIB_STATE_SIZE, "APP_SENTINEL and LIB_STATE overlap")
RAM_DISJOINT(APP_SENTINEL, APP_SENTINEL_SIZE, FM_SCRATCH, FM_SCRATCH_SIZE, "APP_SENTINEL and FM_SCRATCH overlap")
RAM_DISJOINT(APP_SENTINEL, APP_SENTINEL_SIZE, APP_BUF, APP_BUF_SIZE, "APP_SENTINEL and APP_BUF overlap")
RAM_DISJOINT(APP_STAGE, APP_STAGE_SIZE, UI_VARS, UI_VARS_SIZE, "APP_STAGE and UI_VARS overlap")
RAM_DISJOINT(APP_STAGE, APP_STAGE_SIZE, UI_FRAME, UI_FRAME_SIZE, "APP_STAGE and UI_FRAME overlap")
RAM_DISJOINT(APP_STAGE, APP_STAGE_SIZE, SCR_VARS, SCR_VARS_SIZE, "APP_STAGE and SCR_VARS overlap")
RAM_DISJOINT(APP_STAGE, APP_STAGE_SIZE, SCR_STACK, SCR_STACK_SIZE, "APP_STAGE and SCR_STACK overlap")
RAM_DISJOINT(APP_STAGE, APP_STAGE_SIZE, KD_SCRATCH, KD_SCRATCH_SIZE, "APP_STAGE and KD_SCRATCH overlap")
RAM_DISJOINT(APP_STAGE, APP_STAGE_SIZE, FAR_VARS, FAR_VARS_SIZE, "APP_STAGE and FAR_VARS overlap")
RAM_DISJOINT(APP_STAGE, APP_STAGE_SIZE, LIB_STATE, LIB_STATE_SIZE, "APP_STAGE and LIB_STATE overlap")
RAM_DISJOINT(APP_STAGE, APP_STAGE_SIZE, FM_SCRATCH, FM_SCRATCH_SIZE, "APP_STAGE and FM_SCRATCH overlap")
RAM_DISJOINT(APP_STAGE, APP_STAGE_SIZE, APP_BUF, APP_BUF_SIZE, "APP_STAGE and APP_BUF overlap")
RAM_DISJOINT(APP_STAGE, APP_STAGE_SIZE, APP_SENTINEL, APP_SENTINEL_SIZE, "APP_STAGE and APP_SENTINEL overlap")
RAM_DISJOINT(APP_STAGE, APP_STAGE_SIZE, HOME_STATE, HOME_STATE_SIZE, "APP_STAGE and HOME_STATE overlap")
RAM_DISJOINT(APP_STAGE, APP_STAGE_SIZE, HOME_PENDING, HOME_PENDING_LEN, "APP_STAGE and HOME_PENDING overlap")
RAM_DISJOINT(APP_SENTINEL, APP_SENTINEL_SIZE, HOME_STATE, HOME_STATE_SIZE, "APP_SENTINEL and HOME_STATE overlap")
RAM_DISJOINT(APP_SENTINEL, APP_SENTINEL_SIZE, HOME_PENDING, HOME_PENDING_LEN, "APP_SENTINEL and HOME_PENDING overlap")
RAM_DISJOINT(FM_RAM, FM_RAM_SIZE, LIB_BOUNCE, LIB_BOUNCE_SIZE, "FM_RAM and LIB_BOUNCE overlap")

#endif
