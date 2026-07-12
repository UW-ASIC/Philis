---
title: "2.8 Assembly"
chapter: 2
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-2, assembly, packaging, wirebond, leadframe, die-attach]
---

# 2.8 Assembly

> **Chapter 2: Semiconductor Fabrication**

## Key Concepts

After wafer fabrication and interconnection are complete, the finished wafer must still be transformed into individual packaged integrated circuits. This post-fabrication sequence --- collectively called **assembly** --- encompasses wafer probing, dicing, die attachment, wire bonding, packaging, and final testing. These steps are usually performed in a separate facility called an **assembly/test site**, which is physically and organizationally distinct from the wafer fabrication facility (fab).

The assembly process bridges the gap between a working die on a silicon wafer and a finished product that can be soldered onto a printed circuit board. Understanding assembly is important for layout engineers because decisions made at the layout level --- bondpad placement, scribe street design, and die size --- directly constrain and interact with assembly processes. Poor layout choices can cause wire bond shorts, increase packaging stress, or even prevent successful assembly altogether.

### Wafer Probing and Dicing

Before assembly, every die on the wafer is electrically tested in a process called **wafer probing** (also known as wafer sort). A probe card makes contact to the bondpads and probepads of each die, and automated test equipment (ATE) runs a quick functional test. Dies that fail are marked with an ink dot or electronically recorded. Packaging adds cost, so many parts are tested at the wafer level before they are assembled --- this avoids the expense of packaging defective dice.

After probing, the wafer is mounted on a flexible adhesive sheet called **release tape** and cut into individual dice by **sawing**. The sawblade traverses specially designed **scribe streets** approximately $75\ \mu\text{m}$ wide between rows and columns of dice. Key constraints on scribe streets:

- Test structures are often placed inside scribe streets, but the amount of metal and oxide must be limited to avoid fouling the saw.
- **Laser scribing** systems are emerging as an alternative to mechanical sawing; they use a laser to generate defect zones deep within the silicon of the scribe streets, after which gentle flexure of the wafer causes it to fracture along those defect paths. Laser scribing permits narrower scribe streets.

## Important Details

### 2.8.1. Mount and Bond

#### Leadframes

Conventional packages use metal **leadframes** fabricated by stamping or etching a metal sheet. The leadframe provides both a mounting platform (die paddle / mount pad) for the die and the lead fingers that become the external pins of the package.

| Property | Detail |
|---|---|
| **Material** | Copper or copper alloy, plated for corrosion resistance and solder adhesion |
| **CTE mismatch** | Copper's coefficient of thermal expansion (CTE) differs significantly from silicon, causing mechanical stress during thermal cycling |
| **Alternative material** | **Alloy 42** (nickel-iron) --- CTE closer to silicon, but inferior mechanical and electrical properties; used for low-stress specialty packaging |

#### Die Attach Methods

The die must be securely attached to the leadframe's mount pad. Several methods exist, offering different trade-offs between thermal/electrical conductivity, mechanical stress, and cost:

1. **Silver-filled epoxy** --- Most common method. An epoxy resin filled with finely divided silver powder improves thermal conductivity. However, silver epoxy *cannot* be trusted to provide low-resistance electrical connectivity. It is inexpensive and low-stress.

2. **Solder attach** --- The backside of the die is plated with a metal or metal alloy and soldered to the leadframe. Provides excellent thermal and electrical contact, but at higher mechanical stress and cost.

3. **Gold eutectic die attach** --- A rectangle of gold foil (called a **gold preform**) is placed on the leadframe. Heating and scrubbing the die against the gold causes a low-melting alloy (a **eutectic**) to form, welding the two together at a temperature far lower than gold's actual melting point. Excellent thermal and electrical contact.

4. **Silver sintering** --- High-pressure sintering of a silver paste. A newer technique that provides low-resistance thermal and electrical connectivity.

All methods except silver-filled epoxy provide a low-resistance electrical path between the backside of the die and the leadframe. This matters for circuits that use the substrate as a ground or supply connection.

#### Wire Bonding

Wirebonds connect the bondpads on the die surface to the lead fingers of the leadframe. Key terminology:

- **Bondpads**: Openings in the protective overcoat (passivation) large enough to accommodate a bondwire. These are the connection points for both wirebonding and wafer probing.
- **Probepads**: Smaller pads reserved for testing purposes only --- not bonded during assembly.

**Evolution of bonding**: Originally done by human operators under binocular microscopes (slow, error-prone). By the 1980s, the industry transitioned to **automated wire bonding machines** using optical recognition to locate bondpads. Modern machines can bond 10 or more wires per second with great precision.

#### Ball Bonding Process

The dominant technique for gold wire bonding is **ball bonding**. The six-step process is:

1. **Electric flame-off (EFO)**: An electric arc melts the end of the gold wire protruding from the capillary, forming a small ball. (Early machines used a hydrogen flame.)
2. **First bond (ball bond)**: The capillary presses the molten ball down against the bondpad. The gold deforms under pressure and the gold-aluminum interface fuses to create a weld.
3. **Loop formation**: The capillary lifts and moves toward the lead finger, trailing a loop of wire.
4. **Second bond (stitch bond)**: The capillary descends onto the lead finger, smashing the wire against it. The gold alloys to the underlying metal to produce a stitch bond.
5. **Wire break**: The capillary lifts, and the wire breaks at its weakest point.
6. **New ball formation**: A metal wand moves into position and an arc melts the wire protruding from the capillary, forming a new ball for the next cycle.

**Wedge bonding** is an alternative that dispenses with electric flame-off entirely. A small wedge-shaped tool presses the wire against the bondpad and lead finger, creating stitch bonds on both ends of the bondwire.

#### Bondwire Materials

| Material | Notes |
|---|---|
| **Gold** | Historical standard; excellent conductivity, no oxidation, soft enough to avoid damaging circuitry. Standard diameter: 1 mil ($\approx 25\ \mu\text{m}$) |
| **Palladium-coated copper (PCC)** | Replaced gold due to gold's rising price (late 2000s). Palladium coating prevents oxidation. Available down to 0.8 mil ($20\ \mu\text{m}$) |
| **Pure copper** | Used for larger diameters, up to $\sim 5$ mil ($\sim 125\ \mu\text{m}$) |
| **Aluminum** | Large-diameter wedge-bonded wires; used mainly for discrete power transistors. Too soft for IC bonding |

#### Bondpad Sizing and Placement

- Ball bonding requires **square bondpads approximately 2--3 times the wire diameter**. For 1-mil wire, bondpads are about $75\ \mu\text{m}$ across.
- Wedge bonding of small-diameter wires requires similar bondpad dimensions.
- Multiple wires (2 or even 3) can be bonded to a single lead finger for higher current capacity.
- **Proper bondpad placement is critical** to prevent wires from shorting to one another or to the bare sawn edge of the die.
- Proprietary software tools exist to verify that bondwire placement meets rules for a specific bondwire type and leadframe.

### 2.8.2. Packaging

#### Historical Context

Early ICs used packages designed for discrete transistors (TO-3, TO-5 metal cans, TO-220 plastic). These were expensive, hard to use, and limited to about 10 pins. The invention of the 14-pin ceramic **dual-in-line package (DIP)** in 1965 by Forbes, Rice, and Rogers at Fairchild was a breakthrough. Plastic-molded DIPs with 6 to 64 pins dominated the 1970s--1980s. Today, **surface-mount packages** have almost entirely replaced DIPs.

#### Surface-Mount Package Types

| Name | Designator | Configuration |
|---|---|---|
| Small outline transistor package #23 | SOT-23 | Gull wings on two sides |
| Small outline integrated circuit | SOIC | Gull wings on two sides |
| Thin shrunk small outline package | TSSOP | Gull wings on two sides |
| Quad flat pack | QFP | Gull wings on four sides |
| Thin quad flat pack | TQFP | Gull wings on four sides |
| Quad flat pack no lead | QFN | Lands on four sides |
| Micro ball grid array | $\mu$BGA | Rectangular ball array |

**Chip scale packages** (CSPs) are even smaller and see significant use in highly miniaturized portable products like cellphones.

#### Transfer Molding

Plastic-encapsulated packages are created by **transfer molding**:

1. A mold is clamped around the leadframe containing the bonded die.
2. Heated plastic resin (epoxy filled with finely powdered silica) is forced into the mold from below.
3. The plastic wells up and around the die, lifting the bondwires away from it in gentle loops.
4. The mold compound is cured at approximately $175\degree\text{C}$ for about 90 seconds.

The mold compound composition is critical: modern compounds are filled with approximately **90% silica** to minimize the coefficient of thermal expansion (CTE) of the plastic and reduce mechanical stresses on the die. CTE mismatch between the mold compound and silicon is a primary source of **package-induced mechanical stress** on the die, which can shift electrical parameters of sensitive analog circuits (see Chapter 8 for discussion of mechanical stress and package shift).

#### Post-Molding Steps

1. **Lead trimming and forming**: A mechanical press simultaneously trims the links between individual leads and bends them to their required shape.
2. **Lead finishing**: Historically tin-lead solder dipping or plating, but the EU's **RoHS directive** (1996) ended tin-lead solder use. Modern leads are plated with lead-free high-tin solder or a noble metal such as palladium.
3. **Symbolization**: Part numbers and date codes are printed or laser-burned onto the package surface.
4. **Final testing**: A **handler** aligns parts and inserts them into a test socket connected to ATE. If wafer probing was performed, final test may only include rudimentary checks for open/short-circuit bond failures. Otherwise, extensive room-temperature tests are run, sometimes supplemented by high- or low-temperature testing.
5. **Disposition**: Failed units are destroyed to prevent counterfeiting. Passing units are packaged in tubes, trays, or reels for distribution.

## Diagrams

### Figure 2.43 --- Leadframe Strip for an 8-Pin DIP

![[diagrams/ch02-assembly-fig1.png]]

**Caption**: Simplified diagram of a section of a leadframe strip for an 8-pin DIP package. The leadframe shows the die mount pad (center), surrounding lead fingers, and the metal strip connecting everything prior to trimming. The die is attached to the mount pad, and wirebonds connect bondpads on the die to the lead fingers. Although the DIP is now nearly obsolete, very similar leadframe designs are used for modern surface-mount packages.

### Figure 2.44 --- Ball Bonding Process (6 Steps)

![[diagrams/ch02-assembly-fig2.png]]

**Caption**: The six steps of the gold ball bonding process. Step 1: Electric flame-off (EFO) melts the wire tip into a ball. Step 2: The capillary presses the ball onto the bondpad, forming a ball bond. Step 3: The capillary lifts and moves toward the lead finger. Step 4: The capillary presses the wire onto the lead finger, forming a stitch bond. Step 5: The capillary lifts, breaking the wire. Step 6: EFO forms a new ball for the next bond cycle.

## Practical Takeaways

- **Bondpad placement** directly affects assembly yield. Pads must be spaced far enough apart and away from the die edge to prevent wire shorts. Use vendor-supplied design rule checks (DRC) for bondwire verification.
- **Scribe street width** must accommodate the sawblade ($\sim 75\ \mu\text{m}$) and should minimize metal/oxide content to avoid saw fouling. Laser scribing allows narrower streets.
- **Die attach method** matters for analog circuits: if the substrate must serve as a low-impedance ground or supply path, silver-filled epoxy alone is insufficient --- use solder, eutectic, or sintered silver attach.
- **CTE mismatch** between silicon, the leadframe, and the mold compound introduces mechanical stress that can shift transistor parameters. For stress-sensitive analog designs, consider Alloy 42 leadframes or low-stress packaging.
- **Mold compound** with higher silica fill (up to 90%) reduces CTE mismatch and mechanical stress --- this is standard in modern packages but worth verifying for critical analog parts.
- **RoHS compliance** requires lead-free solder and lead-free lead finishes. This is now universal in commercial products.
- **Multiple bondwires per pin** can increase current-handling capacity. Each 1-mil gold wire can carry a large fraction of an amp, depending on length and operating temperature.
- **Wafer probing before assembly** saves cost by identifying defective dice before the expense of die attach, wirebonding, and packaging is incurred.

## Relation to the Bigger Picture

Assembly is the final step that transforms a fabricated silicon wafer into a usable integrated circuit product. For analog layout engineers, assembly considerations impose real constraints on the physical design: bondpad size and placement must accommodate wirebonding equipment, scribe streets must be wide enough for dicing, and die size affects yield and packaging cost. More subtly, the mechanical stresses introduced by packaging --- due to CTE mismatches between silicon, leadframes, and mold compounds --- can shift the electrical parameters of sensitive analog devices. This theme is explored in depth in Chapter 8 (Section 8.2.8, Mechanical Stress and Package Shift), making assembly knowledge essential for understanding why certain analog layout techniques (such as common-centroid geometries and dummy structures) exist. The interconnection system described in [[ch02-interconnection]] provides the bondpads and protective overcoat openings that directly interface with the assembly processes described here.

## See Also
- [[ch02-interconnection]]

