---
title: "5.2 Contamination"
chapter: 5
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-5, contamination, dry-corrosion, mobile-ions, scribe-seal, reliability]
---

# 5.2 Contamination

> **Chapter 5: Failure Mechanisms**

## Key Concepts

Plastic-encapsulated integrated circuits are inherently vulnerable to contamination. Even though modern mold compounds are carefully formulated to resist penetration by external contaminants, **no plastic is truly impregnable**. Contaminants enter through two pathways:

1. **Along the interface** between the metal pins and the plastic encapsulation
2. **Directly through the bulk plastic** itself

Once contaminants reach the die surface, two principal failure mechanisms come into play: **dry corrosion** (which attacks metallization) and **mobile ion contamination** (which shifts MOS threshold voltages). Both mechanisms are time-dependent and accelerated by temperature and moisture, making them critical reliability concerns for any product expected to operate for years.

The reason this matters to layout designers -- rather than being purely a packaging or process concern -- is that the layout determines where protective overcoat openings exist, how scribe seals are constructed, and how close sensitive circuitry sits to potential contamination ingress points. A layout designer who understands these mechanisms can make choices that dramatically improve long-term reliability.

## 5.2.1 Dry Corrosion

### Failure Mechanisms

**Aluminum** will corrode if exposed to moisture and ionic contaminants. **Copper** will corrode if exposed to moisture and oxygen. Only trace amounts of water are needed to initiate this so-called *dry corrosion* -- the term distinguishes it from the wet corrosion seen in bulk materials exposed to liquid water.

Aluminum is highly reactive, but it rapidly forms a dense, impermeable, and strongly adherent $\text{Al}_2\text{O}_3$ layer that shields the metal from further attack. Corrosion can only occur if something **removes this native oxide layer**. Several agents can do this:

- **Phosphosilicate glass (PSG) with >5% phosphorus**: Moisture reacts with phosphosilicates to form phosphorus acid ($\text{H}_3\text{PO}_3$) and phosphoric acid ($\text{H}_3\text{PO}_4$). These acids first attack the aluminum oxide passivation and then the underlying metal. Given enough time, they can corrode entirely through aluminum metallization. The fix is to simultaneously add boron, producing **borophosphosilicate glass (BPSG)**, which has less tendency to corrode aluminum even in the presence of moisture. Many modern processes replace PSG entirely with **nitride overcoats** for even greater corrosion resistance.

- **Halide ions** (chloride, bromide): Common salt (NaCl) provides an abundant source of $\text{Cl}^-$ ions. Moisture seeping into an IC transports chloride ions to the die surface, where they attack exposed aluminum.

- **Polybrominated flame retardants**: Historically used in mold compounds, these decompose above approximately $200\,^\circ\text{C}$, releasing $\text{Br}^-$ ions that cause corrosion problems similar to chlorides. The EU's RoHS directive (2003) restricted polybrominated diphenyl ethers. One manufacturer ironically replaced them with red phosphorus, which led to rapid corrosion-induced failures despite supposedly passivating coatings. Today's mold compounds use neither halogenated organics nor red phosphorus, but remain susceptible to ingress of moisture, oxygen, and chlorides from the environment.

### Protective Overcoat as a Secondary Barrier

All modern ICs are covered with a **protective overcoat (PO)** that acts as a secondary moisture barrier. However, openings must be made for:
- Bondwire attachment to bondpads
- Probe needle access to probe pads
- Fuse access (in some processes)

Additionally, the PO does not cover the die edges. All of these points represent potential pathways for contaminants to reach metallization.

### Preventative Measures (Dry Corrosion)

- **Minimize the number and area of all PO openings.** A production die should not include any openings that are not absolutely necessary.
- **Use a separate test POR layer** for evaluation testpads. When the part is released to production, make a new POR mask to "close" the testpad openings.
- **Metal must overlap bondpad openings** on all sides by an amount sufficient to account for misalignment. The metal bondpads then protect the underlying oxide from moisture and contaminant entry.
- **Fuse openings** should be as small as possible, with no circuitry of any sort (except the fuse itself) within or adjacent to the opening.
- **For copper wire bonding**, specify palladium-coated copper rather than pure copper. The palladium coating minimizes corrosion at high temperatures. This is especially important for finer wire diameters, where corrosion to a given depth has a disproportionately larger effect.

## 5.2.2 Mobile Ion Contamination

### Failure Mechanisms

Most ionic contaminants cannot diffuse through $\text{SiO}_2$ at temperatures below about $500\,^\circ\text{C}$. However, certain **alkali metal ions** -- lithium ($\text{Li}^+$), sodium ($\text{Na}^+$), and potassium ($\text{K}^+$) -- are mobile even at room temperature. Hydrogen ions ($\text{H}^+$) are also mobile in silicon at relatively low temperatures. **Sodium is by far the most troublesome**, as it is encountered almost everywhere (skin contact, chemicals, packaging materials, etc.).

Mobile ion contamination causes **MOS threshold voltages to drift over time**. The mechanism works as follows:

1. Positively charged sodium ions are initially distributed throughout the gate oxide of, say, an NMOS transistor. An equal number of immobile negative charges are also present in the oxide.
2. When a **positive gate bias** is applied, the sodium ions drift toward the negatively biased backgate (the silicon surface), leaving behind the immobile negative charges near the gate electrode.
3. The presence of **positive charge near the channel** decreases the threshold voltage ($V_{th}$) of the NMOS transistor.
4. The magnitude of $\Delta V_{th}$ depends on the amount of sodium present and previous thermal treatment (high temperatures immobilize some fraction of the sodium).
5. The time required for the shift depends on temperature, bias, and materials adjacent to the oxide -- it can be essentially complete in less than a second at high temperatures, or take many hours at room temperature.

This threshold shift **can be temporarily reversed** by baking unbiased devices at elevated temperatures (e.g., $200{-}300\,^\circ\text{C}$) for a short time. Unfortunately, as soon as bias is restored, the threshold begins shifting again.

### Process-Level Countermeasures

Several process-level countermeasures target mobile ion contamination:

1. **Phosphorus gettering in gate oxide**: Manufacturers of metal-gate CMOS stabilized threshold voltages by adding phosphorus to the gate oxide. The phosphorus immobilizes (getters) sodium ions. Early researchers believed sodium ions were electrostatically attracted to nonbridging oxygen atoms attached to phosphorus, though more recent work casts doubt on this specific mechanism. **Drawback**: Phosphorus stabilization eliminated mobile-ion-induced $V_{th}$ shift but introduced a similar shift of its own -- charges associated with phosphorus atoms shift slightly under strong electric fields, a phenomenon called **dielectric polarization** (or **soakage**). The resulting $V_{th}$ shift is highly consistent and small (typically no more than a few millivolts).

2. **Phosphorus-doped polysilicon gates**: A more modern approach. The phosphorus within the polysilicon immobilizes sodium at the oxide-polysilicon interface *without* introducing significant dielectric polarization. This is the preferred approach in poly-gate processes.

3. **Chlorine injection during oxidation**: A gaseous chlorine source (originally $\text{Cl}_2$ or HCl, later replaced by less toxic trichloroethylene/trichloroethane) is injected into the oxidation furnace. Chlorine atoms segregate at oxide-silicon interfaces, where they trap and neutralize mobile ions.

**Modern wafer fabs** do a superb job of eliminating mobile ions from the finished wafer. Poly-gate CMOS transistors manufactured in a modern fab typically exhibit **far less than 1 mV** of threshold shift due to mobile ions. The problem arises *after* fabrication: moisture seeping through the encapsulation can carry sodium ions from the external environment.

### Protective Overcoat for Mobile Ion Defense

The PO plays a vital role in protecting against mobile ion contamination:
- **Silicon nitride** PO: relatively impermeable to mobile ions (blocks them)
- **Phosphorus-doped glass** PO: immobilizes mobile ions (getters them)

Any opening through the PO is a potential route for mobile ions. Design rules require sufficient overlap of metal over bondpad openings to ensure misalignment cannot expose oxide through the nitride opening. Test pad openings should be "closed" on production material using separate PO masks.

**Fuses** that require PO openings represent possible ingress points. They should be placed **well away from sensitive analog circuitry**, especially matched MOS transistors.

### Scribe Seals

**Scribe streets** -- strips of unused silicon between adjacent dice for the saw blade -- represent another path for mobile ions to enter the oxide. Special structures called **scribe seals** placed around the die periphery slow the ingress of contaminants (moisture and mobile ions) into interlevel and field oxides. They also retard cracks from penetrating into active silicon.

#### Single-Level-Metal Scribe Seal

A typical scribe seal for a single-level-metal CMOS process consists of:

1. **A continuous contact ring** surrounding the active die area. This contact must be an uninterrupted ring (no gaps!) so it can block the lateral diffusion of mobile ions through the field oxide. Metal placed over the contact and a P-type diffusion beneath it allow it to **double as a substrate contact** -- the metal plate adds to the width of the substrate ground ring and reduces its resistance.

2. **A protective overcoat flap-down** extending into the scribe street over bare silicon. Mobile ions attempting to penetrate the seal must first surmount this flap-down, then pass the continuous contact ring. Processes using silicon nitride or oxynitride PO normally prohibit direct contact between these materials and bare silicon (mechanical stresses spawn defects), but a special exception is made for the scribe street flap-down because it lies far from active devices.

#### Double-Level-Metal Scribe Seal

For a double-level-metal CMOS process, a third barrier is added:

3. **A continuous via ring** placed just inside the contact ring. This prevents contaminants from diffusing through the interlevel oxide between metal layers. Processes with additional metal layers require via rings between *each pair* of adjacent metal layers. If layout rules permit superimposed contacts and vias, the rings can be stacked.

#### Process-Specific Variations

- **Standard bipolar**: Uses isolation + base diffusion instead of PSD for the substrate contact portion.
- **Dielectric-isolated (DI) processes**: Employ one or more **deep trench isolation rings** as part of their scribe seals. These rings primarily prevent **delamination** between the buried oxide (BOX) and the superficial silicon layer. The BOX interface is relatively weak because wafer bonding does not produce a perfect molecular union. Stresses from sawing and assembly cause delamination that begins at the die edge and proceeds inward. **Large-radius fillets** are applied to corners of isolation rings because sharp corners concentrate stresses.

## Diagrams

### Figure 5.10 -- Mobile Ion Behavior Under Bias

![[diagrams/ch05-contamination-fig1.png]]

**Caption**: Behavior of mobile ions under bias in an NMOS gate oxide. (A) Initially, positively charged sodium ions are randomly distributed throughout the gate oxide, balanced by immobile negative charges. (B) After application of a positive gate bias (+10 V), sodium ions drift toward the negatively biased backgate (silicon surface), shifting the threshold voltage downward. The immobile negative charges remain near the gate electrode.

### Figure 5.11 -- Scribe Seal Structures

![[diagrams/ch05-contamination-fig2.png]]

**Caption**: Scribe seals for CMOS/BiCMOS processes. (A) Single-level-metal: features a continuous contact ring (doubling as substrate contact) plus a PO flap-down into the scribe street. (B) Double-level-metal: adds a continuous via ring between metal layers to block contaminant diffusion through interlevel oxide. The P-epi substrate contact, field oxide blockade, and PO extension work together as multiple barriers against mobile ion ingress.

## Practical Takeaways

- **Minimize all protective overcoat (PO) openings** on production dice. Every opening is a potential contamination ingress point.
- **Use a separate test POR mask layer** for evaluation pads, and "close" those openings on the production POR mask.
- **Ensure metal overlaps bondpad PO openings** on all sides with sufficient margin to account for misalignment -- this prevents moisture from reaching the underlying oxide.
- **Keep fuse openings small** and locate fuses far from sensitive analog circuitry (especially matched MOS transistors).
- **Use palladium-coated copper bondwire** instead of bare copper to minimize corrosion, especially for fine wire diameters.
- **Implement continuous scribe seals** (contact rings with no gaps) around the die periphery to block lateral diffusion of mobile ions through the field oxide.
- **Add via rings** for each additional metal layer in multi-level metal processes.
- **Apply large-radius fillets** at corners of deep trench isolation rings in DI processes to prevent stress-induced delamination.
- **The scribe seal contact ring can double as the substrate ground ring**, saving area and reducing ground ring resistance.
- **For DI processes**, include deep trench isolation rings in the scribe seal to prevent BOX delamination.
- **Remember that mobile ion effects are reversible** (unbiased bake at $200{-}300\,^\circ\text{C}$), but the reversal is temporary -- the shift returns when bias is reapplied.
- **Modern fabs contribute <1 mV of mobile-ion-induced $V_{th}$ shift** from the manufacturing process itself; the real threat is post-fabrication contamination through the package.

## Relation to the Bigger Picture

Section 5.2 sits between the discussion of electrical overstress mechanisms (Section 5.1, covering self-heating, filamentation, electromigration, and TDDB) and surface effects (Section 5.3, covering hot-carrier injection, NBTI, and charge spreading). While electrical overstress concerns the damage caused by excessive voltages or currents during operation, and surface effects concern gradual parametric shifts from charge trapping, contamination concerns the ingress of foreign species (moisture, ions, halides) from the external environment -- a threat that is uniquely sensitive to packaging and layout decisions. The scribe seal and protective overcoat design choices discussed here directly connect to the bondpad and die-edge layout practices covered in later chapters on device matching and mixed-signal layout. For analog IC designers, mobile ion contamination is particularly insidious because it causes slow, bias-dependent $V_{th}$ drift that can degrade precision circuits (voltage references, matched differential pairs, DACs) over the product's lifetime.

## See Also
- [[ch05-electrical-overstress]]
- [[ch05-surface-effects]]
