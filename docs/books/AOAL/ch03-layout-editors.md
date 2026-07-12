---
title: "3.1 Layout Editors"
chapter: 3
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-3, layout-editors, CIF, GDSII, OASIS, CAD]
---

# 3.1 Layout Editors

> **Chapter 3: Layout**

## Key Concepts

The process of creating mask data for an integrated circuit is called **layout** or **physical design**. A software tool called a **layout editor** is used for this purpose. Examples of current-generation layout editors include Cadence's Virtuoso, Mentor Graphic's IC Station, Tanner's L-Edit, and Silvaco's Expert.

### Historical Context

Layout has its roots in the earliest days of IC fabrication. Jack Kilby's first integrated circuit was drawn by hand using photolithography. Early artwork masters were created by hand at magnifications of $100\times$ to $400\times$, with designers carefully charting artwork on sheets of drafting film. These were photographically reduced onto photomasks.

The key evolutionary steps were:

1. **Digitizers** replaced hand drafting (early CAD). Calma Corporation introduced the first layout digitizer. The digitizer converted coordinates punched on cards or magnetic tape into a computer-readable database.
2. **Optical pattern generators** replaced the photographic reduction step -- a pattern generator read data and adjusted its aperture to produce the correct rectangles on glass reticles.
3. **Electron-beam systems** (MEBES by Etec Systems) replaced optical pattern generators in the 1980s for higher precision. The MEBES format remains in use today.
4. **Workstation-based layout editors** emerged in the late 1980s. Calma's GDSII format became the industry standard. Two companies now dominate: **Mentor Graphics** and **Cadence Design Systems**.

Modern layout editors have matured significantly, with supporting tools for autoplace-and-route, verification, parasitic extraction, and more. However, these tools augment rather than replace the basic layout editor.

---

## 3.1.1 Coordinates

Layout editors use a **right-handed two-dimensional Cartesian coordinate system**. Each coordinate consists of an ordered pair of numbers:
- The first number is the displacement along the $X$-axis
- The second is the displacement along the $Y$-axis

When viewed on screen: positive $X$ points right, positive $Y$ points upward. These axes are mapped to the photomask during pattern generation.

### User Units vs. Database Units

Layout editors generally allow a choice of **user units**. Most modern designs use **microns** ($\mu m$). One might expect that user units would correspond to physical distances, but this is actually **not necessarily true** -- the correspondence depends on choices made during pattern generation. Some older designs used units of mils.

If user units corresponded to physical distances, then $1$ mil would equal approximately $25.4\,\mu m$. Whether this is actually true depends on the pattern generation setup.

### Internal Representation and the Database Unit

Layout editors internally store coordinates as **integers** to avoid rounding errors. The editor multiplies each number entered in user units by a constant called the **database-units-per-user-unit ratio** (DBU/UU ratio) and rounds the result to the nearest integer.

For example: if the user entered coordinates of $(11.3, 3.45)$ and the editor was set to $1000$ DBU/UU, it would internally store these coordinates as $(11300, 3450)$.

**Practical implication:** Utilities that translate data from one DBU/UU ratio to another can only perform this operation losslessly if the new ratio is an **integer multiple** of the old ratio. Otherwise, rounding errors can accumulate. Conversion from one user unit to another can present similar difficulties -- if one assumes that one mil equals $25.4$ microns, then converting to a database coded in mils to microns would require multiplying all coordinates by a factor of $25.4$, which can introduce rounding errors. This is why companies historically preferred to equate one mil to $25$ microns rather than $25.4$ microns.

### Key Rules

- Modern layout editors store process-related information in a **technology file** (or tech file). The process designers create a default tech file, and it includes a default choice of user units and DBU/UU ratio. Layout designers should seldom, if ever, change these values.

---

## 3.1.2 The Grid

Modern workstations use mice as pointing devices. The cursor moves across a window displaying a portion of the layout. The layout editor has two important grid concepts:

- **Cursor snapping:** The user moves the cursor to the desired location and presses a button. A feature called **grid snapping** forces the cursor to land only on coordinates that are an integer multiple of a value called the **grid increment**. The user can set the grid increment to any value corresponding to an integer number of database units.

**Example:** When coding in microns at $1000$ DBU/UU, the user can set a grid increment of $0.01$ microns, or $0.001$ microns, but not $0.0001$ microns (since that would be smaller than a single database unit).

- **Coding increment:** A layout designer should always select a grid increment that is an integer multiple of the **coding increment**. The process designers select a coding increment to ensure that coordinates entered on this grid will not experience rounding errors during pattern generation. Coordinates that fall on integer multiples of the coding increment are said to lie on the **minimum grid**. Most layout rules expect all coordinates entered by the user to lie on the minimum grid.

---

## 3.1.3 Shapes

The basic building blocks of layout are **shapes**, also called **figures** or **geometries**. Common types include: **rectangles**, **polygons**, **circles**, and **paths**. Not all editors support all shape types, nor do they all agree on how to interpret them.

Each shape resides on a specific **layer**. One can image these layers as if they were sheets of transparent film stacked one atop another. Shapes intended for different purposes reside on different layers. The shapes on any given layer all receive the same user-selected combination of color, linestyle, and fillstyle.

The layout editor internally refers to layers using **integer layer numbers**, but users refer to them by their corresponding **layer names** (alphanumeric strings). The process development team will assign default layer numbers and names when they construct the tech file. Most users do not need to know the relationship between layer numbers and names, but this becomes important when writing control files for translation utilities.

### Rectangles

The simplest shape is a **rectangle** (also called a **box**). The layout editor internally represents a rectangle by a layer number and two coordinates that define the **lower-left** and **upper-right** vertices.

A rectangle defined by coordinates $(0, 0)$ and $(5, 3)$ has:
- A **boundary** consisting of four line segments connecting its four vertices
- An **interior** (the filled region inside the boundary)

The layout editor draws the boundary using a **linestyle** and fills the interior using a **fillstyle**, both determined by the layer.

### Polygons

A polygon (also called a **boundary** or a **region**) defines a polygonal shape. The editor internally stores a polygon as a layer number and an ordered list of three or more coordinates defining its vertices. The boundary consists of line segments connecting successive vertices, plus one additional segment connecting the last vertex back to the first.

**Orthogonal vs. non-orthogonal:** The sides of an orthogonal polygon are parallel to the horizontal and vertical axes. Orthogonal shapes are sometimes called **Manhattan geometries** (after Brooklyn's famously boxy streets), while non-orthogonal shapes are sometimes called **Brooklyn geometries**. Low-voltage CMOS processes often allow **only orthogonal geometries** because this greatly simplifies the creation and implementation of layout rules. Higher-voltage processes use non-orthogonal geometries to eliminate sharp-cornered shapes that would otherwise intensify electric fields and reduce operating voltages.

**Important constraint:** The boundary of a simple polygon **never intersects itself** (though it may fold back to touch itself). Polygons that touch themselves in this manner are called **nonmonotone polygons**. The boundary of a nonmonotone polygon actually intersects itself. Several algorithms exist for finding the interior and exterior regions of geometries. These algorithms all give the same results when applied to simple and antimonotone figures, but they do not necessarily agree upon how to interpret nonmonotone figures. Therefore, **layouts should never contain nonmonotone geometries**.

### Circles

A layout editor internally stores a circle as a **layer number**, a **radius**, and a **central coordinate**. The editor draws a circular boundary using this information.

**Critical practical issue:** Circles can cause problems because the **MEBES format** traditionally used for mask generation does not support them. Therefore, the pattern generation process decomposes each circle into a **polygonal approximation**. Many layout designers prefer to create their own polygonal approximations of circles so that they know exactly how many sides they contain and exactly where those sides fall.

**Guidelines for circle approximation:**
- Most layout editors include an option to set the number of sides in the polygonal approximation
- One should always use a **multiple of four** to ensure that the resulting figure displays both horizontal and vertical symmetry
- This precaution allows one to rotate or reflect circles without interfering with their matching
- Popular choices for the number of sides in a circle include $32$ and $64$

### Paths

Paths (also called **centerline figures** or **wires**) are extremely useful for drawing metallization. A layout editor internally stores a path as a layer number, a width, and an ordered list of two or more coordinates. These coordinates define the **centerline** of the path.

The boundary of the path has the specified width and follows the specified centerline. One can find the boundary of a path by erecting two construction lines parallel to each segment of the centerline spaced away from it by half the width. An additional construction line passes through the last vertex and lies perpendicular to the last centerline segment. The vertices of the boundary lie at the intersections of the construction lines.

**Width rule:** The width of a path should always equal an integer multiple of **twice the coding increment**. This ensures that the sides of an orthogonal path fall on the minimum grid. Most design rules specify the minimum widths of metal and poly layers as multiples of twice the coding increment. One can use this fact to guess the coding increment of a process based upon knowledge of its design rules.

**Example:** If all the design rules are multiples of $0.04\,\mu m$, one can safely guess that the coding increment is $0.020\,\mu m$ and not $0.04\,\mu m$ because the metal width should be a multiple of twice the coding increment.

**Rounding errors:** The boundary of a nonorthogonal path contains vertices that fall off the minimum grid. Rounding errors during pattern generation can slightly reduce the width of diagonal path segments, and in the case of minimum-width paths, this can lead to layout-rule violations. Traditionally, process developers have solved this problem either by prohibiting nonorthogonal paths, or by padding the design rules to cover any anticipated shortfall. Some newer layout editors offer an option that changes the way path boundaries are generated to ensure that their vertices fall on the minimum grid.

**Path end styles (Figure 3.5):**
- **Flush-ended (default):** The ends of the path intersect the first and last vertices of the centerline
- **Extended path:** Ends extend half the width beyond the first and last centerline vertices
- **Rounded path:** Ends formed by arcs centered on the first and last centerline vertices

Most layout designers avoid using rounded paths because different tools draw rounded paths in somewhat different ways.

**Zero-width paths:** Some layout editors also allow zero-width paths, consisting of a centerline without a boundary. Zero-width paths have no interior and thus generate no mask data. Editors that do not support zero-width paths generally offer an alternative shape called a **line**.

---

## 3.1.4 Hierarchy

Layout editors store shapes in collections called **cells**. Each cell has a unique name. The user can create cells, copy them, edit them, or delete them. A **layout database** consists of a collection of cells used in a specific design. All cells in the database share the same tech file, use the same layer definitions, user units, and so forth. However, each cell contains its own collection of shapes.

### Instances and Instantiation

A cell can also contain one or more **instances** of other cells. Each such instance contains:
- A single coordinate (the **origin**)
- A **cell name** (identifying which cell is being referenced)

When the layout editor encounters an instance, it references the named cell and draws whatever it finds inside. This process, known as **instantiation**, greatly simplifies the creation of repetitive structures such as logic gates. Instead of having to draw each gate separately, one can draw a single gate and then place instances of it wherever desired.

### Instance Behavior and Complications

**All instances share a single master cell.** Suppose a certain layout requires six copies of a comparator. The layout designer creates a single comparator cell and places six instances of it. Now suppose that she must edit one instance of this comparator. Her edits affect **all** six locations simultaneously. An edit that fits nicely in one location might create a **metal short** elsewhere. This problem can be avoided by copying the cell, instantiating the copy, and editing it.

**Nested hierarchy:** An instance of a cell can contain instances of other cells, and those cells can contain instances of yet more cells. In this manner, one can construct a complex hierarchical structure called a **tree**. Rather confusingly, most designers imagine this tree growing **downward** rather than upward, and the root of the tree is therefore called the **top cell**. The top cell represents the entire integrated circuit.

### Transformations

Various transformations can be applied to instances, including **rotation**, **reflection**, and **magnification**. Many editors support only the eight so-called **orthogonal transformations** (or **Manhattan transformations**):

| Code | Transformation |
|------|---------------|
| R0 | Default orientation (no transformation) |
| R90 | Rotate 90 degrees counterclockwise |
| R180 | Rotate 180 degrees |
| R270 | Rotate 270 degrees counterclockwise |
| MX | Reflect about the X-axis |
| MY | Reflect about the Y-axis |
| MX90 | First reflect about X-axis, then rotate 90 degrees |
| MY90 | First reflect about Y-axis, then rotate 90 degrees |

A small diamond denotes the location of each instance. R0 draws the instance in its default orientation. R90, R180, and R270 rotate the instance counterclockwise by successive increments of $90°$. MX reflects the placement about the $X$-axis, while MY reflects it about the $Y$-axis. MX90 and MY90 first perform the indicated reflections and then rotate the figure counterclockwise by $90°$. These transformations are **orthogonal** in the sense that they never transform orthogonal data into nonorthogonal data.

Some layout editors support magnification and nonorthogonal (any-angle) rotation. These features can easily cause rounding errors, and most layout rules do not permit their use even if layout editors implement them. However, these operations are admittedly useful for constructing complicated shapes.

### Arrayed Instances

Layout editors also support **arrayed instances**. Each consists of an instance name, a single coordinate, two integers representing the number of rows and columns in the array, and two real numbers representing the spacing between rows and columns. Arrayed instances were originally intended for creating digital registers and memories, but they are also useful for creating rectangular arrays of contacts and vias. Transformations applied to an arrayed instance usually reference the origin of the **lower leftmost** instance. For example, an array rotates around the origin of its lower leftmost instance.

### Pins

A **schematic** is an abstract graphical representation of an electrical circuit consisting of idealized electrical devices connected together by idealized wires. A **netlist** is a text file containing the same information in the form of a series of statements about electrical devices.

A layout designer can create a layout cell that corresponds to the schematic. A special shape called a **pin** is used to represent the pin of the cell. The layout editor internally stores a pin as a polygon and a layer number to represent the pin's location. In addition, it stores a text string that represents the **pin name**. Most layout editors also make provisions for storing additional properties, for example, pin direction. The traditional choices of pin directions are **input**, **output**, and **input/output**. This information is used to check for wiring errors. Circuit simulators will report an error whenever two output pins are connected together by a wire in the schematic.

### Parameterized Cells (Pcells)

Most modern layout editors support **parameterized cells**, or **Pcells**. When the user places an instance of a Pcell, the editor requests values for one or more parameters. For example, when placing a Pcell of a MOS transistor, the user might need to enter its width and length. The values of these parameters are stored separately for each instance, so changing the parameters of one instance will not affect others.

**Pcell capabilities** include:
- Stretch a group of objects a specified distance in the $X$-direction, the $Y$-direction, or both
- Include or exclude a group of objects
- Repeat a group of objects in the $X$-direction, the $Y$-direction, or both
- Modify shapes placed during the creation of an instance
- Repeat objects along the border of a parameterized shape
- Place objects relative to a reference point

Pcells are usually created by writing software subroutines and binding them to specific instances, but some layout editors also include graphical tools that allow one to create Pcells without having to write code.

**Benefits and limitations:**
- A library of tested and proven Pcells greatly reduces the time and effort required to create layouts
- The dimensions within a properly designed Pcell always pass design rule checks, which greatly reduces the amount of time spent verifying layouts
- Furthermore, if the Pcells embody good design practices, then so will the layouts that use them
- **Limitation:** Pcells function only within the environment of the layout editor that created them. If one must transfer a design from one layout editor to another, each instance of a Pcell becomes an instance of a unique cell containing shapes corresponding to one specific set of Pcell parameters. These "exploded" Pcells have no connection to one another; if a change must be made to them, each and every one must be opened and edited.
- **Version compatibility:** Problems with Pcells occur when the process development team fixes an error or makes an improvement. The translator may need to update the database, but users must then ask: "Should I let the translator change the schematic to match the existing layout, or should I let it change the layout to match the existing schematic?" Both choices pose problems.

**Best practice:** Expert layout designers will occasionally need to create custom layout instead of Pcells. In general, one shouldn't replace a Pcell simply because it doesn't look neat or because this would save a trifling amount of space. On the other hand, if using the Pcell would significantly degrade performance, then a custom layout should be used instead. Generally one should start by instantiating a Pcell that resembles the desired layout as closely as possible, then explode and edit it. Additional steps may be required to ensure that a specific layout editor properly recognizes the exploded data. When using Cadence VXL, one should place the exploded data inside a cell, check to make sure that pins have been properly placed, and then add the schematic instance to reference this layout cell.

---

## 3.1.5 Interchange File Formats

Many companies make layout tools, but no two of them agree on exactly how they should work. Users want to transfer data from one tool to another, and even between tools made by different vendors. Therefore, several file formats have been developed to support data interchange:

| Format | Description |
|--------|-------------|
| **CIF** | Caltech Intermediate Format |
| **GDSII** | Calma's GDSII stream format |
| **OASIS** | Open Artwork System Interchange Standard |

Since interchange formats do not support proprietary features such as Pcells, layout editors must convert such data into more primitive objects before writing the interchange file.

### CIF (Caltech Intermediate Format)

CIF was originally developed by **John Ousterhout** and **Ron Ayres** at $1979$, but the version usually supported by modern tools is **CIF Version 2.0**. Before GDSII became the de facto industry standard, many manufacturers used CIF. Some tools still support CIF even though it is arguably rather dated.

**CIF properties:**
- Supports: rectangles (called **boxes**), polygons, circles (called **flashes**), paths (called **wires**), and instances (called **calls**)
- **Does NOT support:** array instances, pins, and text
- Coordinates are represented as **signed integers** (originally supposed to be $25$-bit signed integers, but modern utilities generally support $32$-bit integers)
- CIF always represents data in microns at $100$ DBU/UU
- Layers are specified by **name** rather than by number; any text string can be used as a name as long as it does not include a semicolon
- Layer names can include spaces
- CIF boxes can be rotated to any angle; the rotation occurs about the center of the box
- Polygons have no set maximum number of vertices, and nonconvex figures are explicitly permitted
- Radians are specified by **x-over-y** and **diameter**
- Wires are encoded ends by default, although nothing stops an application from interpreting the centerline data differently
- Cells (symbols) are identified by **number** rather than by name
- Cells can receive a rotation vector that supports **any-angle rotation**, but **not magnification**

**CIF is a textual format** with no explicit line-length limitation, but the specification suggests lines should be limited to $132$ characters. Continuation across lines is allowed.

### GDSII (Calma Stream Format)

GDSII was created by the Calma Company to store information created using its GDSII layout editor. GDSII quickly gained acceptance as an interchange format because it represents most types of data in a relatively compact and easy-to-process format.

- The most commonly encountered version is **Release 6.0**, published by Calma in $1987$
- Calma made a slight extension (GDSII Version 7) to support their $6250$ editor in $1987$; this had no effect on others using the format
- GDSII stream format is currently owned by **Cadence Design Systems**, who have trademarked it and loosened a few restrictions

**GDSII properties:**
- **Binary format** that supports polygons (called **boundaries**), paths, instances (called **structure references**), and arrayed instances (called **array references**)
- Coordinates are represented as **32-bit signed integers**. The file includes fields that define user units and database units in terms of meters, thus allowing any choice of user units and DBU/UU ratio
- GDSII **does not support circles**, but it does include a way to represent text
- Files are segmented into **records**; data is properly delimited via record and datestamp information
- Wires are encoded ends by default
- GDSII originally supported only $64$ layers, numbered from $0$ to $63$. However, each shape can also receive a **datatype** which can also assume any value from $0$ to $63$. These datatypes effectively function as sublayers. Most modern utilities extend the range of layers and datatypes to $0$-$255$
- **GDSII does not store layer names**, so conversion utilities usually require a control file that maps the tool's internal layer names into GDSII layers and datatypes

**GDSII path types:**
| Type | Description |
|------|-------------|
| Type 0 | Flush ends |
| Type 1 | Has rounded ends |
| Type 2 | Extends both ends by half the width |
| Type 4 | Extended ends with variable amounts |

All tools recognize path type $0$ and must recognize path type $1$; the others may or may not be recognized by a green tool. **Zero-width paths are allowed.**

**Structures** are referenced by name. Names may contain up to $32$ characters consisting of A through Z, $0$ through $9$, underscores, question marks, and dollar signs. Most users **avoid question marks and dollar signs** because other tools may not recognize them as valid characters.

**Limitations:**
- Structure references, array references, and text can be rotated to **any angle** or magnified by any real positive magneto value
- Angles are measured counterclockwise in degrees
- A magnification of $2.0$ would double all dimensions measured from the origin, while a magnification of $0.5$ would halve all dimensions
- Rounding errors may occur during processing because both angles and magnifications are stored as real numbers
- Most tools **do not support** any-angle rotation and magnification, so **these features should be used with caution**
- Array references specify a stepping distance in $X$ and $Y$, and a number of rows and columns, each of which may range from $1$ to $32767$
- The entire array rotates counterclockwise around the origin of the lower leftmost instance

As previously mentioned, Cadence has relaxed some of the limitations specified above. They now allow $4,096$ vertices in boundaries and paths; layer and datatype numbers from $0$ to $32,767$; and structure names containing up to $63,534$ characters. Other vendors may or may not follow these guidelines.

### OASIS (Open Artwork System Interchange Standard)

OASIS was developed by **Semiconductor Equipment and Materials International (SEMI)** beginning in $2001$. OASIS seeks to achieve at least an order of magnitude file size reduction compared to GDSII through a combination of:
- Signed repetition patterns
- Delta operations
- Internal data compression

OASIS has not yet gained widespread acceptance, but it is the only open-source standard that currently serves a viable replacement for GDSII.

**OASIS properties:**
- **Binary format** that defines shapes not by vertices, but rather by **delta operations** that specify displacements relative to a previous location
- Different types of delta operators are defined for orthogonal and nonorthogonal geometries, including one type specifically intended for rectangular figures
- Coordinates and deltas are stored as **signed integer quantities of arbitrary length**
- User units are microns, and the format includes provisions for specifying any desired DBU/UU ratio
- A variety of **repetition operators** allow compact specification of various repetitive patterns of shapes without using instantiation
- OASIS supports layers and datatypes in essentially the same manner as GDSII, but **does not limit the number** of either
- However, like GDSII, it **does not directly support layer names**

**OASIS shapes:**
- Rectangles, polygons, circles, paths, and several other specialized types such as **trapezoids**
- Supports instances, arrayed instances, and text
- Rectangles are specified by width, height, and repetition pattern
- Polygons have no limitation upon the number of allowed vertices
- Although nonconvex figures are not prohibited, the standard does not greatly assist in their recognition
- Circles are represented by center, radius, and repetition pattern
- Paths are specified by half-width, centerline points, and repetition pattern
- Three path types exist: flush ends, ends extended by a half width, and ends extended by arbitrary amounts
- **Zero-width paths are allowed**

**OASIS references cells by name.** The lengths of these names are unlimited and any printable ASCII characters except spaces and tabs are allowed inside names. Instances (called **placements**) include support for any-angle rotation and magnification. The use of a repetition pattern can transform any instance into any of a wide variety of arrayed instances.

---

## Diagrams

### Figure 3.1 -- Rectangles and Polygons
![[diagrams/ch03-layout-editors-fig1.png]]
*Shows the fundamental shapes used in layout editors. A rectangle is defined by its lower-left vertex at $(0,0)$ and upper-right vertex at $(5,3)$, with labeled boundary and interior. A polygon is defined by an ordered list of vertices connected by line segments, with small squares marking each vertex.*

### Figure 3.6 -- The Eight Orthogonal Transformations
![[diagrams/ch03-layout-editors-fig2.png]]
*Illustrates the eight orthogonal (Manhattan) transformations supported by most layout editors: R0 (identity), R90, R180, R270 (rotations), MX (reflect about X-axis), MY (reflect about Y-axis), MX90 (reflect then rotate 90 degrees), and MY90 (reflect then rotate 90 degrees). These transformations preserve orthogonal geometry -- they never convert Manhattan shapes into non-Manhattan shapes.*

### Figure 3.7 -- Schematic, Netlist, and Pins
![[diagrams/ch03-layout-editors-fig3.png]]
*Shows a two-input NAND gate schematic with its corresponding SPICE netlist. This illustrates the concept of pins: the schematic identifies nodes (A, B, Y, VDD, VSS) that must have corresponding pin shapes in the layout cell. The netlist defines the .subcircuit with pin names that the layout must match for LVS verification.*

---

## Practical Takeaways

- **Always use the default tech file settings** for user units and DBU/UU ratio. Changing these can introduce subtle rounding errors during data translation.
- **Path widths must be integer multiples of twice the coding increment** to ensure both edges of the path fall on the minimum grid.
- **Never create nonmonotone polygons** -- different algorithms interpret them differently, leading to unpredictable results.
- **Approximate circles with polygons using a multiple of four sides** (typically $32$ or $64$) to preserve horizontal and vertical symmetry under rotation and reflection.
- **Avoid rounded path ends** -- different tools interpret them differently. Use flush or extended ends instead.
- **Avoid question marks and dollar signs in cell names** for GDSII compatibility.
- **Use caution with any-angle rotation and magnification** in GDSII -- these features can cause rounding errors since angles and magnifications are stored as floating-point numbers.
- **When creating custom layouts instead of Pcells**, start by instantiating the closest matching Pcell, then explode and edit it, rather than building from scratch. This preserves design rule compliance.
- **Pcell limitations across tools:** When transferring designs between different layout editors, Pcells are "exploded" into static geometry. Any future parameter changes require manual editing of every instance.
- **Prefer GDSII over CIF** for interchange -- GDSII is the de facto standard. CIF lacks support for arrays, pins, and text. Consider OASIS for very large designs where file size matters (order-of-magnitude reduction over GDSII).
- **Grid discipline is paramount:** All coordinates should fall on the minimum grid (integer multiples of the coding increment). Violations create subtle mask-generation errors that are extremely difficult to debug.

---

## Relation to the Bigger Picture

Section 3.1 establishes the foundational vocabulary and mechanics of the layout editor -- the primary tool that analog layout designers will use throughout the remainder of the book. Understanding coordinates, grids, shapes, hierarchy, and interchange formats is essential prerequisite knowledge before tackling the design rules covered in [[ch03-design-rules]] (Section 3.2), because design rules are expressed in terms of the very constructs defined here (layer names, minimum widths, spacings measured in user units on the coding increment grid). The hierarchical cell/instance model introduced here also directly underpins the practical layout techniques for transistors, resistors, capacitors, and full analog blocks covered in later chapters, where judicious use of Pcells and hierarchical instantiation is critical for maintaining matching and reducing layout errors.

---

## See Also
- [[ch03-design-rules]]
