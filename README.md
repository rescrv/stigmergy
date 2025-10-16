# Stigmergy: Emergent Coordination through Dynamic Components and Systems

Stigmergy represents a fundamental departure from traditional software architectures by implementing biological principles of emergent coordination in computational systems. Drawing inspiration from the ways termites construct complex mounds and ants optimize foraging trails without centralized control, this framework demonstrates how sophisticated global behaviors can arise from simple local interactions mediated through a shared data environment. The architecture eschews rigid class hierarchies and compile-time bindings in favor of a fully dynamic entity-component-system model where components and systems are defined, composed, and coordinated at runtime through data-driven specifications.

At the heart of stigmergy lies the recognition that complex software systems can be understood as collections of autonomous agents acting upon shared data structures, with coordination emerging naturally from the traces these agents leave in the environment. Unlike traditional object-oriented systems where behavior is tightly coupled to data through class definitions, or functional systems where computation flows through predetermined pipelines, stigmergy enables coordination through environmental modification. Systems observe component data attached to entities, evaluate whether they should act based on bidding expressions, and modify the shared state through precisely scoped component access permissions. This separation of data, identity, and behavior creates an architecture of remarkable flexibility and composability.

## The Entity-Component-System Foundation

The architectural foundation of stigmergy rests upon three carefully separated concerns: entities that provide identity, components that carry data, and systems that embody behavior. Entities are cryptographically strong 32-byte identifiers encoded as URL-safe base64 strings with an "entity:" prefix, serving as stable anchors in the computational environment around which data and behavior coalesce. These identifiers carry no intrinsic meaning or type information; they are pure names in the most fundamental sense, gaining significance only through the components attached to them and the systems that operate upon those components. This radical separation of identity from type represents a deliberate philosophical stance: in stigmergy, what something *is* emerges from what properties it *has* and what operations can *act* upon it, rather than being determined by inheritance hierarchies or interface declarations.

Components constitute the primary mechanism for attaching typed data to entities, each component being a named type following Rust-style path conventions paired with JSON data validated against a schema. The component system supports arbitrary nesting through JSON object and array types, discriminated unions through the oneOf schema construct, and optional fields through standard JSON schema validation patterns. Critically, components are defined at runtime rather than compile-time; applications dynamically register new component types by submitting schemas to the system, which then enforces validation for all instances of that component type. This runtime definition of component schemas enables the system to evolve without recompilation, accommodating new data structures as they emerge from the needs of the domain rather than requiring them to be anticipated during initial system design.

Systems in stigmergy are autonomous agents that evaluate whether they should act upon entities based on component data, expressing their interest through bidding expressions and operating through carefully scoped component access. Each system is defined by a markdown file containing YAML-like frontmatter that specifies its configuration, including a name, description, model specification, display color, list of component access permissions, and list of bid expressions. The component access specifications employ a capability-based security model where systems explicitly declare whether they require read, write, or execute access to each component type, ensuring that systems can only observe and modify the precise subset of the shared environment necessary for their operation. This fine-grained access control creates a computational analog to the way biological organisms in stigmergic systems can only perceive and modify their immediate local environment.

## Dynamic Component Architecture

The dynamic component architecture represents a synthesis of schema validation, type safety, and runtime flexibility that allows applications to evolve their data models without system downtime or recompilation. Component definitions pair a component type name with a JSON schema that specifies the structure and constraints of valid component data, supporting primitive types including null, boolean, integer, number, and string, as well as complex types including objects with typed properties and arrays with typed elements. The schema validation system implements full recursion for nested structures, enabling the definition of arbitrarily complex data types such as trees, graphs, and hierarchical configurations. The oneOf construct provides discriminated unions that allow a single component type to represent multiple variants, each with its own schema, enabling polymorphic data modeling within the component framework.

Component instances represent the materialization of component schemas as concrete data attached to specific entities, with each instance validated against its component's schema before creation and on every subsequent update. The validation system operates as a gatekeeper that enforces data integrity across the entire distributed environment, ensuring that all component instances adhere to their declared schemas regardless of which client or system created or modified them. This validation layer provides formal guarantees about data structure and constraints that hold across system boundaries, creating islands of type safety in the broader ocean of untyped JSON data. The combination of runtime schema definition and rigorous validation achieves a remarkable balance: the system gains the flexibility of dynamic typing with the safety guarantees typically associated with static type systems.

The automatic schema generation system bridges the gap between Rust's strong static type system and stigmergy's dynamic component architecture through a derive macro that generates JSON schemas from Rust type definitions. Struct definitions with typed fields generate object schemas with corresponding property types, while enum definitions generate either string enums for simple variants or oneOf discriminated unions for variants with associated data. Optional fields in Rust structs are represented in the JSON schema by omitting them from the required array, allowing instances to include or exclude those properties as needed. This bidirectional mapping between Rust types and JSON schemas enables developers to define component structures using familiar type definition syntax while still maintaining the runtime flexibility that makes stigmergy's dynamic architecture possible.

## System Specification and Bidding

Systems in stigmergy are specified through markdown documents with YAML-like frontmatter that declare their configuration, capabilities, and interests in a human-readable format amenable to version control and collaborative editing. The frontmatter section contains required fields including system name, description, model specification, and display color, along with optional lists of component access specifications and bid expressions. The markdown content following the frontmatter provides detailed instructions for the system's operation, typically including context about its purpose, descriptions of how it uses the components to which it has access, and examples of its expected behavior. This separation of machine-readable configuration from human-readable documentation creates living specifications that serve simultaneously as executable definitions and as comprehensible descriptions of system behavior.

Component access specifications in system configurations employ a declarative syntax that explicitly enumerates which component types a system can observe or modify, along with the mode of access required. Each specification consists of a component type name followed by an access mode of read, write, execute, or read+write, establishing a capability that constrains the system's interaction with the environment. Read access permits a system to observe component data when evaluating whether to bid on an entity, write access permits modification of component data during system execution, and execute access permits invocation of component methods for components that represent tools or operations. The explicit declaration of access requirements creates a computational analog to the physical constraints that determine which environmental traces biological organisms can perceive and modify in natural stigmergic systems.

Bid expressions constitute the mechanism through which systems declare their interest in acting upon specific entities based on component data, with each expression consisting of a condition clause and a value clause separated by the BID keyword. The condition clause specifies a boolean expression over component fields using familiar programming language operators including comparison (==, !=, <, <=, >, >=), logical (&&, ||, !), arithmetic (+, -, *, /, %, ^), and pattern matching (~= for regular expressions). When a system evaluates its bids against an entity, it checks whether the entity has the necessary components and whether the condition evaluates to true; if so, it computes the value expression and submits that value as its bid. The value expression can reference component fields, perform arithmetic computations, and incorporate system-specific parameters, enabling sophisticated bidding strategies that account for entity state, system capabilities, and environmental conditions.

The bidding mechanism implements a coordination protocol whereby multiple systems can express interest in acting upon the same entity, with the auction mechanism selecting the system with the highest bid value to perform the operation. When an entity possesses components that multiple systems can observe, each system evaluates all of its bid expressions against that entity's component data and submits the maximum bid value it computed. The auction mechanism then selects the winning system and invokes it with access to the entity's components according to the access specifications in its configuration. This competitive bidding protocol creates emergent prioritization of system actions without requiring explicit coordination between systems; systems implicitly coordinate by competing for the opportunity to act, with the bidding expressions encoding their relative priorities and capabilities.

## Runtime Composition and Emergent Behavior

The true power of stigmergy's architecture emerges from the ability to compose systems and components at runtime without modifying existing definitions, enabling the creation of novel entity types and behaviors through pure data composition. An entity can have any combination of components attached to it, limited only by uniqueness (each entity can have at most one instance of each component type) and schema validation (each component instance must conform to its type's schema). This compositional freedom means that new entity types can be created simply by attaching different combinations of existing components, without requiring the definition of new classes, the implementation of new interfaces, or the compilation of new code. The Pokevania example in the repository demonstrates this principle by creating hybrid entities that combine components from both Pokemon and Castlevania domains, achieving emergent gameplay that neither system anticipated.

Systems can be defined, modified, and deployed independently of one another because they interact only through the shared component environment and the auction mechanism that coordinates their execution. A new system can be introduced to an existing stigmergy deployment simply by submitting its configuration to the system management API; that system will immediately begin evaluating bids against existing entities that possess the components it declares access to. Similarly, an existing system can be modified by updating its configuration, changing its bid expressions to express different priorities or modifying its component access requirements to operate on different data. This hot-swappable nature of system definitions enables continuous evolution of the computational environment without downtime, analogous to how biological stigmergic systems can adapt their behavior as individual organisms learn new responses to environmental traces.

The compositional architecture creates emergent behaviors that arise from the interaction of independently defined systems operating on shared component data according to their own local criteria. Consider an entity with Health, Position, and Healer components: the healing-aura-system bids on this entity because it has Healer and damaged Health, while a hypothetical movement-system might bid because it has Position and a destination. The auction mechanism coordinates these competing interests by selecting the highest bidder, but the key insight is that neither system needed to know about the other during its definition. The coordination emerges from the interaction of their bid expressions with the shared component data, creating a form of implicit message passing where the messages are component modifications and the routing mechanism is the bidding protocol.

## Persistence, Distribution, and Scalability

The persistence layer grounds stigmergy's dynamic architecture in PostgreSQL, providing ACID guarantees for entity and component operations while enabling efficient queries across the component space. Entities are stored with automatic timestamp tracking for creation and modification events, while components are stored with foreign key relationships to both their owning entities and their component definitions. The schema validation performed at the application layer is complemented by relational integrity constraints at the database layer, ensuring that component instances always reference valid entities and component definitions. This dual-layer validation strategy provides defense in depth: the JSON schema validation ensures structural correctness of component data, while the relational constraints ensure referential integrity of the entity-component graph.

The HTTP API exposes stigmergy's capabilities through RESTful endpoints that support creation, retrieval, update, and deletion operations for entities, components, component definitions, and systems. Entity endpoints provide operations for generating new entity identifiers and querying entity metadata, while component endpoints enable attaching, retrieving, updating, and removing component instances from entities. Component definition endpoints support registering new component types with their schemas and retrieving schema information for validation and introspection. System endpoints allow defining new systems from markdown specifications, updating system configurations, and querying the list of available systems. This comprehensive API enables stigmergy to serve as a coordination substrate for distributed applications where multiple clients create and manipulate entities according to their own local logic.

The architectural separation of entities, components, systems, and the bidding mechanism creates natural opportunities for horizontal scaling and distributed deployment. Different component types can be stored in different database instances through sharding strategies that partition entities based on their component composition or based on identifier ranges. Systems can be deployed as independent services that query for entities matching their component access requirements, evaluate their bids, and submit bid values to a central auction coordinator. The auction mechanism itself can be distributed across multiple coordinators that partition the entity space and communicate through consensus protocols. This distributed architecture mirrors the way biological stigmergic systems scale across large spatial extents and numerous organisms without requiring centralized coordination or global state.

## Theoretical Foundations and Future Directions

Stigmergy as a computational architecture instantiates several deep theoretical ideas from distributed systems, programming languages, and artificial intelligence research. The entity-component-system model implements a form of prototype-based programming where entities gain capabilities through component composition rather than class inheritance, similar to languages like Self and JavaScript but with the addition of schema validation and access control. The bidding mechanism implements a form of capability-based security where systems must explicitly declare their access requirements, combined with a market-like coordination protocol reminiscent of multi-agent systems research. The dynamic definition of components and systems creates a reflective system where the structure of the computational environment can be modified from within that environment, enabling meta-programming and self-modification capabilities.

The biological metaphor of stigmergy suggests several directions for future research and development. Pheromone-like components could carry time-decay properties that cause their values to diminish over time unless reinforced, enabling temporal coordination patterns where recent actions have stronger effects than historical ones. Gradient-following systems could compute spatial derivatives over component fields to create directed behaviors analogous to chemotaxis in biological systems. Multi-entity systems could evaluate bids based on patterns across multiple entities simultaneously, enabling collective behaviors that emerge from population-level dynamics. These extensions would move stigmergy closer to its biological inspiration while maintaining the principled separation of concerns that makes the current architecture tractable and analyzable.

From a software engineering perspective, stigmergy represents an exploration of how much structure can be made dynamic without sacrificing correctness guarantees or system comprehensibility. The dynamic component schemas ensure that the system cannot enter invalid states despite the lack of compile-time type checking, while the explicit component access specifications document system dependencies in machine-readable form. The markdown-based system specifications serve as living documentation that remains synchronized with system behavior by virtue of being the source of that behavior. These properties suggest that dynamic architectures need not sacrifice the benefits of static typing and explicit documentation when appropriate runtime abstractions provide equivalent guarantees through different mechanisms.

## Installation and Operation

The stigmergy system requires PostgreSQL for persistence and Cargo for building the Rust implementation. Installation proceeds through standard Rust toolchain commands: cargo build compiles the system, cargo test executes the test suite, and cargo clippy performs static analysis. The stigmergyd binary serves the HTTP API and coordinates system execution, while stigctl provides a command-line interface for interacting with a running stigmergy deployment. Database migrations establish the necessary schema in PostgreSQL, creating tables for entities, components, component definitions, systems, and invariants along with appropriate indices for efficient querying.

Configuration occurs through environment variables and command-line arguments processed through the arrrg argument parsing library. Database connection strings specify the PostgreSQL instance for persistence, HTTP port numbers configure the API endpoint, and optional logging parameters control diagnostic output verbosity. Systems are deployed by submitting their markdown specifications through the HTTP API or the stigctl command-line tool, while component definitions are registered either through explicit API calls with JSON schemas or through automatic schema generation from Rust type definitions. Running examples populate the database with sample entities and components that demonstrate stigmergy's capabilities, including the Pokemon, Castlevania, and hybrid Pokevania scenarios documented in the examples directory.

## Demonstrations and Examples

The examples directory contains three comprehensive demonstrations that showcase stigmergy's composition capabilities:

1. **Pokemon** (`examples/pokemon/`) - A Pokemon-like game world demonstrating classic ECS patterns for turn-based RPG mechanics with components like `PokemonSpecies`, `Trainer`, and `Party`.

2. **Castlevania** (`examples/castelvania/`) - A gothic action-adventure world inspired by Castlevania, showcasing different game mechanics with components like `HunterProfile`, `MonsterProfile`, and `Arsenal`.

3. **Pokevania** (`examples/pokevania/`) - A hybrid world that merges Pokemon and Castlevania mechanics without requiring any new component definitions, demonstrating how emergent gameplay arises from component composition. This example showcases the true power of ECS: entities can simultaneously have Pokemon and Castlevania components (e.g., Hunter-Trainers, Gothic Pokemon), creating novel gameplay from existing systems.

The examples directory also contains system definitions for healing auras, hunger mechanics, damage over time, shield regeneration, and resurrection systems. Each example system specifies its component access requirements and bid expressions in markdown format, providing both executable system definitions and human-readable documentation of system behavior. The example entities combine these components in various ways to create emergent interactions: a being with both Healer and Health components will heal itself and nearby allies, while a being with Hunger and Health components will gradually take damage from starvation. These examples illustrate how complex behaviors emerge from the composition of simple, independently defined systems operating on shared component data through the coordination of the bidding mechanism.

See `examples/README.md` for detailed setup instructions and documentation for each demo.

## Command-Line Interface: stigctl

The `stigctl` command-line tool provides a comprehensive interface for interacting with a running stigmergy server. It enables creation, retrieval, update, and deletion operations for all core stigmergy primitives: entities, components, component definitions, systems, and invariants.

### Global Options

```bash
stigctl [options] <command> [args...]
```

Options:
- `--base-url <url>` - Base URL of the Stigmergy API server (default: http://localhost:8080)
- `--output <format>` - Output format for get/list commands: json or yaml (default: json)

### Entity Commands

Entities are the fundamental identifiers in stigmergy. Each entity is a cryptographically strong 32-byte identifier that serves as an anchor for component data.

```bash
# Create a new entity (generates a unique entity identifier)
stigctl entity create

# List all entities
stigctl entity list

# Delete an entity
stigctl entity delete <entity-id>
```

Example:
```bash
ENTITY=$(stigctl entity create | awk '{print $3}')
echo "Created entity: $ENTITY"
```

### Component Definition Commands

Component definitions establish schemas that validate component instances. They pair a component type name with a JSON schema that specifies structure and constraints.

```bash
# Create a component definition with a JSON schema
stigctl componentdefinition create <name> <schema>

# List all component definitions
stigctl componentdefinition list

# Get a component definition by name
stigctl componentdefinition get <name>

# Update a component definition's schema
stigctl componentdefinition update <name> <schema>

# Delete a component definition
stigctl componentdefinition delete <name>
```

Example:
```bash
# Create a Health component definition
stigctl componentdefinition create "game::Health" '{
  "type": "object",
  "properties": {
    "current": {"type": "integer", "minimum": 0},
    "maximum": {"type": "integer", "minimum": 1}
  },
  "required": ["current", "maximum"]
}'
```

### Component Commands

Components attach typed, validated data to entities. Each component instance must conform to its component definition's schema.

```bash
# Create a component instance for an entity
stigctl component create <entity-id> <component-name> <data-json>

# List all components attached to an entity
stigctl component list <entity-id>

# Get a specific component instance
stigctl component get <entity-id> <component-name>

# Update a component instance
stigctl component update <entity-id> <component-name> <data-json>

# Delete a component instance from an entity
stigctl component delete <entity-id> <component-name>
```

Example:
```bash
# Attach a Health component to an entity
stigctl component create $ENTITY "game::Health" '{"current": 80, "maximum": 100}'

# Update the health values
stigctl component update $ENTITY "game::Health" '{"current": 100, "maximum": 100}'

# List all components on the entity
stigctl component list $ENTITY
```

### System Commands

Systems are autonomous agents defined by markdown files containing configuration frontmatter and operational instructions. They evaluate bid expressions to determine which entities to act upon and operate through scoped component access permissions.

```bash
# Create a system from JSON config
stigctl system create <config-json>

# Create a system from a markdown file (recommended)
stigctl system create-from-md <file.md>

# List all systems
stigctl system list

# Get a system by name
stigctl system get <system-name>

# Update a system
stigctl system update <system-name> <config-json>

# Delete a system
stigctl system delete <system-name>
```

Example:
```bash
# Create a system from a markdown file
stigctl system create-from-md examples/healing-aura-system.md

# List all registered systems
stigctl system list

# Get the full system definition
stigctl system get healing-aura

# Output in YAML format
stigctl --output yaml system get healing-aura
```

### Invariant Commands

Invariants are expressions that must hold true across the system. They provide runtime validation of system-wide constraints.

```bash
# Create an invariant with an optional custom ID
stigctl invariant create <expression> [id]

# List all invariants
stigctl invariant list

# Get an invariant by ID
stigctl invariant get <invariant-id>

# Update an invariant
stigctl invariant update <invariant-id> <expression>

# Delete an invariant
stigctl invariant delete <invariant-id>
```

### Typical Workflows

#### Creating a New Entity Type

```bash
# 1. Define component schemas
stigctl componentdefinition create "game::Position" '{
  "type": "object",
  "properties": {
    "x": {"type": "number"},
    "y": {"type": "number"}
  },
  "required": ["x", "y"]
}'

stigctl componentdefinition create "game::Velocity" '{
  "type": "object",
  "properties": {
    "dx": {"type": "number"},
    "dy": {"type": "number"}
  },
  "required": ["dx", "dy"]
}'

# 2. Create an entity
ENTITY=$(stigctl entity create | awk '{print $3}')

# 3. Attach components to compose the entity type
stigctl component create $ENTITY "game::Position" '{"x": 10.0, "y": 20.0}'
stigctl component create $ENTITY "game::Velocity" '{"dx": 1.5, "dy": -0.5}'

# 4. View the composed entity
stigctl component list $ENTITY --output yaml
```

#### Deploying a System

```bash
# 1. Create a system markdown file (examples/my-system.md)
# (Define name, description, model, color, component access, and bid expressions)

# 2. Deploy the system
stigctl system create-from-md examples/my-system.md

# 3. Verify deployment
stigctl system list

# 4. Inspect system configuration
stigctl system get my-system --output yaml
```

#### Querying and Debugging

```bash
# List all entities to see what exists
stigctl entity list

# Examine components attached to a specific entity
stigctl component list <entity-id> --output yaml

# View all component definitions (schemas)
stigctl componentdefinition list --output yaml

# Check registered systems
stigctl system list

# Get detailed system configuration
stigctl system get <system-name> --output yaml
```

### Running stigctl During Development

During development, you can run stigctl directly from the source tree using cargo:

```bash
cargo run --bin stigctl -- entity create
cargo run --bin stigctl -- entity list
cargo run --bin stigctl -- system create-from-md examples/healing-aura-system.md
```

For production deployments, build and install stigctl:

```bash
cargo build --release --bin stigctl
sudo cp target/release/stigctl /usr/local/bin/
```
