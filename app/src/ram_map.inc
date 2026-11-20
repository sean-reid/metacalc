; The RAM map: the regions of RAM the kernel, the kernel driver, the UI, the reader and the cross-page call use, in
; address order. The probes keep scratch outside it and check it against the regions of what they link.
; Each region is NAME for its base, NAME_SIZE for its size, and can be moved with -DNAME=$address.
; The file that owns a region lays out the bytes inside it. Assembling any image that includes
; this file fails when two regions overlap or a region leaves its bank.
;
;   name              bank  where
;   SCR_STATE         B     the stack of open screens and a few variables, 160 bytes at the start
;                           of appData; the OS leaves it alone through _GetKey, auto power down
;                           included, and the first 256 bytes are free
;   UI_VARS           B     the UI's variables, in saveSScreen; the OS zeroes it when auto power
;                           down fires in _GetKey
;   UI_FRAME          B     the UI's 96 by 64 frame, plotSScreen
;   KD_SCRATCH        B     the kernel driver's scratch, from libPage, in appBackUpScreen
;   FAR_VARS          B     the cross-page call's scratch (app/src/farcall.inc), in the gap
;                           between KD_SCRATCH and LIB_STATE
;   LIB_STATE         B     the reader's state, after the driver's scratch
;   KERNEL_RAM_LOW    C     the kernel's low region
;   KERNEL_RAM_HIGH   C     the kernel's high region
;   LIB_BOUNCE        C     an overlay on the last 256 bytes of the kernel's high region
;   FM_SCRATCH        B     fm's 256 byte buffer for the reader's accessors, in saveSScreen after
;                           UI_VARS; it holds nothing across calls
;   FM_RAM            C     fm's working memory, an overlay on the kernel's high region from its
;                           start; the kernel uses that region only inside a kernel call, so fm
;                           and the kernel share it through the ready mark (fm/fm.z80)
;   FM_LOW            C     fm's second working region, an overlay on the kernel's low region from
;                           KERNEL_RAM_LOW + $100 to its end, shared with the kernel the same way;
;                           the page below it holds the kernel's parameters and ready mark and fm
;                           never writes it
#ifndef RAM_MAP
#define RAM_MAP

#ifndef SCR_STATE
SCR_STATE           .equ $8000
#endif
#ifndef SCR_STATE_SIZE
SCR_STATE_SIZE      .equ 160
#endif
#ifndef UI_VARS
UI_VARS             .equ $86EC
#endif
#ifndef UI_VARS_SIZE
UI_VARS_SIZE        .equ $1A
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

RAM_WITHIN(SCR_STATE, SCR_STATE_SIZE, $8000, $C000, "SCR_STATE runs past bank B")
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
RAM_DISJOINT(SCR_STATE, SCR_STATE_SIZE, UI_VARS, UI_VARS_SIZE, "SCR_STATE and UI_VARS overlap")
RAM_DISJOINT(SCR_STATE, SCR_STATE_SIZE, UI_FRAME, UI_FRAME_SIZE, "SCR_STATE and UI_FRAME overlap")
RAM_DISJOINT(SCR_STATE, SCR_STATE_SIZE, KD_SCRATCH, KD_SCRATCH_SIZE, "SCR_STATE and KD_SCRATCH overlap")
RAM_DISJOINT(SCR_STATE, SCR_STATE_SIZE, FAR_VARS, FAR_VARS_SIZE, "SCR_STATE and FAR_VARS overlap")
RAM_DISJOINT(SCR_STATE, SCR_STATE_SIZE, LIB_STATE, LIB_STATE_SIZE, "SCR_STATE and LIB_STATE overlap")
RAM_DISJOINT(KERNEL_RAM_LOW, LOW_SIZE, KERNEL_RAM_HIGH, HIGH_SIZE, "KERNEL_RAM_LOW and KERNEL_RAM_HIGH overlap")

RAM_WITHIN(FM_SCRATCH, FM_SCRATCH_SIZE, $8000, $C000, "FM_SCRATCH runs past bank B")
RAM_WITHIN(FM_RAM, FM_RAM_SIZE, KERNEL_RAM_HIGH, KERNEL_RAM_HIGH + HIGH_SIZE, "FM_RAM runs outside the kernel's high region")
#if FM_LOW < (KERNEL_RAM_LOW + $100)
 .error "FM_LOW must not start below KERNEL_RAM_LOW + $100, the page of the kernel's parameters"
#endif
RAM_WITHIN(FM_LOW, FM_LOW_SIZE, KERNEL_RAM_LOW, KERNEL_RAM_LOW + LOW_SIZE, "FM_LOW runs outside the kernel's low region")
RAM_DISJOINT(FM_SCRATCH, FM_SCRATCH_SIZE, SCR_STATE, SCR_STATE_SIZE, "FM_SCRATCH and SCR_STATE overlap")
RAM_DISJOINT(FM_SCRATCH, FM_SCRATCH_SIZE, UI_VARS, UI_VARS_SIZE, "FM_SCRATCH and UI_VARS overlap")
RAM_DISJOINT(FM_SCRATCH, FM_SCRATCH_SIZE, UI_FRAME, UI_FRAME_SIZE, "FM_SCRATCH and UI_FRAME overlap")
RAM_DISJOINT(FM_SCRATCH, FM_SCRATCH_SIZE, KD_SCRATCH, KD_SCRATCH_SIZE, "FM_SCRATCH and KD_SCRATCH overlap")
RAM_DISJOINT(FM_SCRATCH, FM_SCRATCH_SIZE, FAR_VARS, FAR_VARS_SIZE, "FM_SCRATCH and FAR_VARS overlap")
RAM_DISJOINT(FM_SCRATCH, FM_SCRATCH_SIZE, LIB_STATE, LIB_STATE_SIZE, "FM_SCRATCH and LIB_STATE overlap")
RAM_DISJOINT(FM_RAM, FM_RAM_SIZE, LIB_BOUNCE, LIB_BOUNCE_SIZE, "FM_RAM and LIB_BOUNCE overlap")

#endif
