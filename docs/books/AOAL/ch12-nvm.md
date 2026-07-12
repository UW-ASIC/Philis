---
title: "12.3 Nonvolatile Memory"
chapter: 12
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-12, NVM, EPROM, EEPROM, floating-gate]
---

# 12.3 Nonvolatile Memory

> **Chapter 12: Field-Effect Transistors**

## Key Concepts

Nonvolatile memory (NVM) in analog IC design allows circuits to retain calibration data, trim settings, and user configuration across power cycles. Unlike digital products that use dense flash memory (requiring several extra mask steps), analog products typically need only a few bits of NVM, so the emphasis is on cells compatible with **baseline analog CMOS and BiCMOS processes** -- no special process extensions.

There are two fundamental programming mechanisms for floating-gate devices:

### Hot Carrier Injection (EPROM)

A floating-gate transistor is programmed by operating it in saturation under a high drain-to-source voltage. A small fraction of channel hot electrons generated in the pinched-off region scatter off the silicon lattice and acquire enough energy to surmount the Si-SiO$_2$ barrier. These electrons accumulate on the electrically isolated floating gate. The key requirement is that the **gate-to-source voltage must approximately equal the drain-to-source voltage** -- if $V_{GS} < V_{DS}$, the vertical electric field actually *repels* carriers away from the oxide interface. Programming therefore requires substantial drain current (on the order of 100 $\mu$A per bit) and an external programming voltage pin, since on-chip charge pumps cannot supply enough current.

### Fowler-Nordheim Tunneling (EEPROM)

Fowler-Nordheim tunneling allows both injection and removal of electrons through a thin tunnel oxide. The tunneling current through an oxide of area $A$ is:

$$I = J_0 \cdot A \cdot \exp\!\left(-\frac{E_{crit}}{E_{ox}}\right) \tag{12.37}$$

where $J_0$ is the rate constant (typically ~$10^6$ A/cm$^2$), $E_{crit}$ is the critical electric field (~$250$ MV/m), and $E_{ox}$ is the applied oxide field. Programming typically uses an oxide field of ~$10$ MV/m, yielding a tunneling current of ~$10^{-6}$ A/cm$^2$. This requires negligible current compared to hot-carrier injection, making it possible to generate the programming voltage on-chip using a charge pump.

### The Floating-Gate Principle

The core idea behind all NVM devices in this section is the **floating gate** -- a polysilicon electrode that is completely surrounded by insulating oxide, with no DC path to any circuit node. Charge stored on this gate shifts the effective threshold voltage of the underlying transistor:

$$V_{T,eff} = V_T + \frac{Q_f \cdot (C_1 + C_2)}{C_1 \cdot C_2} \tag{12.36}$$

where $Q_f$ is the charge on the floating gate, $C_1$ is the capacitance between the floating gate and the backgate (through gate oxide), and $C_2$ is the capacitance between the floating gate and the control gate (through interlevel oxide). When the device is erased ($Q_f = 0$), a channel forms at a control-gate voltage of:

$$V_{CG} = V_T \cdot \frac{C_1 + C_2}{C_2} \tag{12.35}$$

The double-poly transistor therefore initially acts as an NMOS transistor with a somewhat larger-than-expected threshold voltage. After programming, the accumulated negative charge $Q_f$ further increases the effective threshold, turning the device into a **normally-closed switch that opens when programmed**.

## The Double-Poly EPROM Transistor (FLOTOX)

The standard double-poly EPROM transistor has two gate electrodes: a **floating lower gate** and an **upper control gate**. The floating gate capacitively couples to the backgate through the gate oxide ($C_1$) and to the control gate through the interlevel oxide ($C_2$). The voltage on the floating gate relative to the backgate is determined by capacitive division between these two capacitors.

The **FLOTOX** (Floating Gate Tunneling Oxide) transistor extends this concept for electrical erasability. It resembles the double-poly transistor, but a portion of the oxide beneath the floating gate is made particularly thin -- this **tunneling oxide** resides over an extension of the transistor's drain.

- **Programming**: Hold drain at ground, apply high voltage to control gate. Electrons tunnel from drain across thin tunnel oxide to floating gate. Negative charge increases $V_{T,eff}$.
- **Erasing**: Hold control gate at ground, apply high voltage to drain. Electrons tunnel from floating gate to drain, removing negative charge and decreasing $V_{T,eff}$.

The FLOTOX transistor forms the basis for all modern EEPROM memories, but requires several special processing steps (extended drain, tunneling oxide, control gate) that are hard to justify for just a few bytes of NVM in an analog IC.

## Oxide Thickness Constraints for Tunneling

Thin gate oxides are the enabling technology for EEPROM. The program/erase voltage is approximately **250% of the maximum operating voltage** for the gate oxide. Key constraints:

- Tunneling oxides **cannot be made thinner than ~60 A (6 nm)** or trap-assisted tunneling limits data retention times.
- Oxides rated for less than ~2.5 V operating voltage cannot serve as tunneling oxides.
- Most modern analog CMOS processes use a ~50 A I/O gate oxide suitable as a tunneling oxide.
- A typical ~70 A gate oxide requires programming voltages of ~12-14 V.

## Write Endurance and Degradation

All floating-gate devices degrade after repeated programming and erasure. Hot carriers passing through the oxide generate **trap sites** that eventually form a **percolation path** (see Section 5.1.4). Once this path forms, charge on the floating gate leaks away rapidly and the transistor no longer functions as a memory element.

- Small EEPROM memories typically achieve write endurance of **>100,000 cycles**.
- In practice, analog EPROM is often characterized for only ~1,000 write cycles due to testing cost, even though it could do far more.
- The number of permissible write cycles depends statistically on the number of bits in the memory -- more bits means a higher chance that at least one cell fails early.

**Critical design rule**: The programming voltage should NOT be applied as a step waveform. Instead, use a **linear ramp** from ground to full voltage at a controlled slew rate (e.g., $10$ V/ms). Failure to observe this precaution can reduce write endurance to just a couple of cycles. A slower ramp produces smaller gate current, increasing write endurance at the cost of longer programming time. Since analog applications seldom require rapid programming, ramp rates of ~$10$ V/s are often employed.

## 12.3.2 Single-Poly EPROM

The simplest EPROM cell consists of nothing more than a **single PMOS transistor** with no connection to its gate electrode, fabricated in a baseline analog CMOS process.

### Process Requirements

- Gate oxide must be at least ~70 A thick (thinner oxides exhibit excessive trap-assisted tunneling).
- Transistors must be able to operate, at least briefly, at the required programming voltage (~12 V for a 70 A oxide).
- **Drain-extended transistors** can handle such voltages but consume considerable die area.

### How It Works

The EPROM transistor is a small PMOS with a floating (unconnected) gate:

1. **Erased state**: The final anneal during fabrication removes any charges accumulated during manufacture. If there is doubt, an erase bake for a few minutes at ~250 C ensures full erasure. The erased device remains in **cutoff** (OFF).
2. **Programming**: The drain-to-source voltage is raised until the drain-backgate junction **avalanches**. Hot electrons deflected into the gate oxide cause negative charge to accumulate on the floating gate. This negative charge induces a channel in the PMOS transistor (turns it ON).
3. **Reading**: Once programmed, the transistor conducts (saturation or linear mode). The read state is the inverse of EPROM logic -- a **normally-open switch that closes when programmed**.

Most analog products use opaque mold compounds that preclude UV erasure, so plastic-encapsulated EPROM effectively becomes **one-time programmable (OTP)** memory.

### Typical Circuit (Figure 12.39B)

A single bit of EPROM memory in an analog application includes:
- An EPROM transistor $M_E$
- A drain-extended NMOS $M_D$ (rated for the full programming voltage)
- A programming transistor $M_P$
- A read transistor $M_R$ forming part of a current mirror biased by a reference transistor and current source
- Transistors forming a simple comparator to determine the cell state during readback

**Programming**: VPP rail is raised to programming voltage, EN and WR inputs go high. The drain-extended transistor handles the full programming voltage appearing at the drain once the EPROM transistor conducts. Each cell draws ~$80$ $\mu$A of programming current. Simultaneously writing 128 cells would require >10 mA -- if the VPP supply cannot handle this, cells can be written in smaller groups.

**Reading**: VPP is set to a low value (~half the operating voltage of the EPROM transistor, e.g., ~5 V for a 12 V EPROM). EN is high, WR remains low. A current mirror attempts to pull a small current from the EPROM transistor. If programmed, it conducts and pulls the sense node low; if erased, the node stays high. The read voltage must be small enough to ensure **absolutely no hot carrier generation** occurs. Good practice: operate in read mode only momentarily and store results in a volatile SR latch.

### Redundancy for Reliability

Replace a single EPROM transistor with **two EPROM transistors connected in parallel**, with gate electrodes NOT connected to each other. If one transistor's gate oxide contains a defect causing charge leakage, the other almost certainly remains unaffected. This vastly increases reliability at the cost of doubling the programming current -- a small price that many designers routinely pay.

## 12.3.3 Single-Poly EEPROM

EEPROM memory is both electrically programmable and electrically erasable. It requires far less current to program than EPROM, and the programming voltage can be generated using an **integrated charge pump**. EEPROM is thus favored over EPROM for most field-programmable devices.

### Cell Structure (Figure 12.40)

One bit of single-poly EEPROM memory consists of three components sharing a **common floating gate**:

| Component | Implementation | Purpose |
|-----------|---------------|---------|
| Tunnel capacitor $C_T$ | Small PMOS transistor in its own N-well | Provides thin-oxide path for Fowler-Nordheim tunneling |
| Control capacitor $C_C$ | Large PMOS transistor in its own N-well | Couples control voltage to floating gate |
| Sense transistor $M_S$ | Normal NMOS transistor | Reads the stored state |

The tunnel capacitor is made **as small as possible**. The control capacitor is **deliberately enlarged**. The ratio $C_C / C_T$ typically equals **at least 20**. This large ratio ensures that most of the voltage differential between the tunneling input and the control input appears across the dielectric of the tunnel capacitor, enabling efficient tunneling.

### Programming, Reading, and Erasing

**Programming** (Figure 12.41A):
- Place programming voltage VPP on the tunneling input $T$.
- Ground the control input $C$.
- Sense transistor source/drain can be grounded or left floating.
- Because $C_C \gg C_T$, most of the voltage appears across the tunnel capacitor dielectric.
- Electrons tunnel **from the floating gate to the tunneling input**, leaving a net positive charge on the floating gate.
- VPP should be a linear ramp (not a step) to maximize write endurance. A ramp rate of ~$10$ V/s is typical for analog applications.

**Reading** (Figure 12.41B):
- Ground both the tunneling input and the control input.
- The positive charge on the floating gate induces a channel in the sense transistor.
- The cell behaves as a **normally-open switch that closes during programming**.

**Erasing** (Figure 12.41C):
- Place high voltage on the control input $C$.
- Ground the tunneling input $T$.
- Sense transistor source/drain can be grounded or left floating.
- Electrons tunnel **to the floating gate from the tunneling input**, replacing positive charge with negative charge.
- When read again, the negative charge suppresses channel formation in the sense transistor (OFF).

### Latched EEPROM Cell (Figure 12.42)

For improved reliability, two EEPROM elements can be interconnected to form a **latched EEPROM cell**:
- Two sense transistors $M_{S1}$ and $M_{S2}$ share a common grounded source.
- Separate drain connections go to a latch circuit.
- Control and gate electrodes are wired so that **programming one transistor erases the other**, and vice versa.
- This minimizes area and programming complexity while retaining redundancy benefits.

### Variation: Tunneling Through the Sense Transistor

Some designs eliminate the separate tunnel capacitor and use the sense transistor itself as the tunneling element. This saves space but complicates the programming, erasing, and reading circuitry.

## Diagrams

### Figure 12.36/12.37 -- FAMOS Transistor and Double-Poly EPROM

![[diagrams/ch12-nvm-fig1.png]]

*Cross section of a FAMOS transistor showing hot electron injection from the avalanching drain-backgate junction onto the floating gate. Below it begins Figure 12.37, the double-poly EPROM transistor with separate floating gate and control gate. The FAMOS transistor is a PMOS whose floating gate accumulates negative charge during programming, inducing a channel (normally-open switch that closes when programmed).*

### Figure 12.39 -- Single-Poly EPROM Cell Layout and Circuit

![[diagrams/ch12-nvm-fig2.png]]

*Layout of a single-poly EPROM cell (A) showing the minimal PMOS transistor with unconnected gate, and the typical analog readback circuit (B) including drain-extended NMOS, programming transistor, read transistor, and comparator. The circuit shows the VPP, EN, and WR control signals needed for programming and reading.*

### Figure 12.40 -- Single-Poly EEPROM Cell Layout and Circuit

![[diagrams/ch12-nvm-fig3.png]]

*Layout of a single-poly EEPROM cell (A) showing the tunnel capacitor (small), control capacitor (large), and sense transistor sharing a common floating polysilicon gate. The tunnel and control capacitors are PMOS transistors, each in its own N-well. The schematic (B) shows how the control input C and tunneling input T connect, with the $C_C/C_T$ ratio of at least 20 ensuring efficient tunneling.*

## Practical Takeaways

- **Process compatibility**: Single-poly EPROM and EEPROM cells can be built in baseline analog CMOS/BiCMOS processes with no extra mask steps -- a critical advantage for analog ICs needing only a few bits of NVM.
- **Avoid metal over EPROM transistors**: Metal generates parasitic capacitances that may alter retention time and prevents UV erasure (useful for wafer probe even on plastic-packaged parts).
- **Use redundant transistors**: Two parallel EPROM transistors with separate floating gates vastly improve reliability for minimal cost (doubled programming current). For EEPROM, use the latched cell topology.
- **Ramp the programming voltage**: Never apply a step waveform. Use a controlled linear ramp (e.g., 10 V/s for analog applications) to maximize write endurance. Failure to do this can reduce endurance to just a couple of cycles.
- **Minimize read time**: Operate the cell in read mode only momentarily, then store the result in an SR latch. Use a read voltage no more than half the EPROM transistor's operating voltage to avoid any hot carrier generation.
- **Capacitor ratio matters in EEPROM**: The control-to-tunnel capacitance ratio ($C_C/C_T \geq 20$) is essential for directing the programming voltage across the tunnel oxide.
- **Gate oxide thickness floor**: Tunneling oxides cannot be thinner than ~60 A (6 nm) due to trap-assisted tunneling degrading data retention. The I/O gate oxide in most modern CMOS processes (~50 A) is suitable.
- **Reference layouts are sacred**: Process designers develop and extensively test reference layouts for EEPROM core devices. The production layout must **exactly match** these qualified layouts to inherit their proven retention and endurance specifications.
- **Programming current budget**: Each EPROM cell draws ~80 $\mu$A during programming. Plan the VPP supply capacity accordingly when programming multiple cells simultaneously, or write cells in smaller groups.
- **EEPROM is preferred for field-programmable devices**: Its negligible programming current (generated on-chip via charge pump) and electrical erasability make it superior to EPROM for applications requiring in-system reprogramming.

## Relation to the Bigger Picture

Section 12.3 bridges the gap between standard MOSFET construction (covered in [[ch12-constructing-cmos]]) and specialized analog transistor structures. While most of Chapter 12 addresses how to build and optimize transistors for analog signal processing, this section shows how the same CMOS fabrication steps can be repurposed to create memory elements critical for analog IC trimming, calibration storage, and user configuration. The floating-gate concept relies on the same gate oxide physics and hot-carrier phenomena discussed earlier in the book (Sections 5.1.4 and 12.3.1), tying together process reliability, device physics, and practical layout. The subsequent section on JFETs ([[ch12-jfet]]) continues the theme of leveraging existing process layers to build specialized devices beyond standard CMOS transistors.

## See Also
- [[ch12-constructing-cmos]]
- [[ch12-jfet]]
