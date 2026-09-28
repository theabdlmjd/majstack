# Architecture

majstack turns a high-level goal into verified software changes. It is an orchestration engine:
real state, real execution, real verification, real recovery, and real evidence.

## 1. System context

```mermaid
flowchart TB
    user([Developer])
    subgraph repo[Your repository]
        cli["majstack CLI"]
        daemon["majstackd daemon"]
        state[("State<br/>.majstack SQLite")]
        assets["Skills · Workflows · Principles"]
    end
    subgraph exec[Execution environments]
        agent["Coding agent CLIs<br/>claude · codex · opencode · custom"]
        typed["Typed decision models<br/>hosted or local OpenAI-compatible"]
        browser["Browser via Chrome DevTools Protocol"]
        gh["GitHub"]
        vcs["Git"]
    end
    user --> cli
    user --> daemon
    cli --> state
    cli --> assets
    daemon --> state
    cli --> agent
    cli --> typed
    cli --> browser
    cli --> gh
    cli --> vcs
    daemon --> agent
```

## 2. Component graph

```mermaid
flowchart TB
    subgraph apps[Applications]
        cli["majstack-cli"]
        daemon["majstack-daemon"]
    end
    subgraph orchestration[Orchestration]
        orch["majstack-orchestration<br/>state machine · scheduler · executor"]
        recovery["majstack-recovery"]
        workflow["majstack-workflow<br/>router"]
        skills["majstack-skills"]
        agents["majstack-agents"]
    end
    subgraph data[State and knowledge]
        state["majstack-state"]
        events["majstack-events"]
        dag["majstack-graph"]
        memory["majstack-memory"]
        evidence["majstack-evidence"]
        telemetry["majstack-telemetry"]
    end
    subgraph execution[Execution]
        providers["majstack-providers"]
        typed["majstack-typed"]
        tools["majstack-tools"]
        execution_rt["majstack-execution"]
        git["majstack-git"]
        github["majstack-github"]
        browser["majstack-browser"]
        verification["majstack-verification"]
    end
    subgraph foundation[Foundation]
        core["majstack-core"]
        config["majstack-config"]
        policy["majstack-policy"]
        logging["majstack-logging"]
        testing["majstack-testing"]
    end
    cli --> orch
    daemon --> orch
    orch --> recovery
    orch --> workflow
    orch --> skills
    orch --> agents
    orch --> dag
    orch --> events
    orch --> memory
    orch --> evidence
    orch --> providers
    orch --> verification
    orchestration --> state
    workflow --> skills
    providers --> typed
    providers --> execution_rt
    tools --> policy
    tools --> state
    git --> execution_rt
    github --> execution_rt
    browser --> execution_rt
    orchestration --> foundation
    data --> core
    execution --> core
```

## 3. Run state machine

```mermaid
stateDiagram-v2
    [*] --> requested
    requested --> understanding: classify goal
    understanding --> investigating: investigation-first work
    understanding --> specifying
    investigating --> specifying
    specifying --> reviewing_spec
    reviewing_spec --> planning
    planning --> reviewing_plan
    reviewing_plan --> decomposing
    reviewing_plan --> planning: reject plan
    decomposing --> ready
    ready --> executing
    executing --> verifying: task attempt done
    verifying --> reviewing: checks pass
    verifying --> repairing: checks fail
    reviewing --> completed: approved & all resolved
    reviewing --> repairing: rejected
    repairing --> reverifying
    reverifying --> reviewing
    executing --> paused: stop requested
    executing --> blocked: no ready tasks
    executing --> needs_human_input: recovery blocking
    executing --> failed: unrecoverable
    paused --> executing: resume
    blocked --> executing: replan
    completed --> [*]
    failed --> [*]
    cancelled --> [*]
```

## 4. One autonomous run (sequence)

```mermaid
sequenceDiagram
    autonumber
    actor U as Developer
    participant CLI as majstack CLI
    participant ENG as Orchestration
    participant DB as SQLite
    participant AG as Coding agent
    participant V as Verification
    U->>CLI: majstack run "goal"
    CLI->>ENG: run_goal(goal)
    ENG->>DB: create run, goal, feature
    ENG->>AG: specification prompt
    AG-->>ENG: specification
    ENG->>DB: persist spec, journal
    ENG->>AG: planning prompt
    AG-->>ENG: tasks JSON
    ENG->>DB: persist plan, tasks, dependencies
    loop until all tasks resolved
        ENG->>DB: select ready task (DAG)
        ENG->>AG: task prompt (fresh context)
        AG-->>ENG: work + sigils
        ENG->>V: run checks
        V-->>ENG: pass/fail + evidence
        alt checks pass
            ENG->>AG: review prompt
            AG-->>ENG: verdict
            ENG->>DB: commit, mark complete, unblock
        else checks fail
            ENG->>DB: classify failure, recovery plan
        end
    end
    ENG->>DB: finalize run, decisions, journal
    ENG-->>CLI: outcome
    CLI-->>U: status, evidence, next steps
```

## 5. Persistence schema

```mermaid
erDiagram
    RUNS ||--o{ TASKS : contains
    RUNS ||--o{ GOALS : targets
    RUNS ||--o{ RUN_JOURNAL : logs
    RUNS ||--o{ DECISIONS : records
    RUNS ||--o{ FAILURES : records
    RUNS ||--o{ EVENTS : emits
    RUNS ||--o{ PROVIDER_RUNS : invokes
    RUNS ||--o{ COMMITS : produces
    FEATURES ||--o{ RUNS : scopes
    TASKS ||--o{ TASK_ATTEMPTS : attempts
    TASKS ||--o{ TASK_DEPENDENCIES : depends
    TASKS ||--o{ EVIDENCE : proves
    TASKS ||--o{ REVIEW_FINDINGS : reviews
    TASKS ||--o{ VERIFICATION_RESULTS : verifies
    PLANS ||--o{ TASKS : decomposes
    GOALS ||--o{ SPECIFICATIONS : specifies
    GOALS ||--o{ PLANS : plans
    RUNS ||--o{ PRINCIPLE_INVOCATIONS : applies
    PROVIDER_RUNS ||--o{ MODEL_RUNS : uses
    RUNS ||--o{ TOOL_CALLS : audits
    RUNS ||--o{ BROWSER_SESSIONS : browses
    BROWSER_SESSIONS ||--o{ BROWSER_ARTIFACTS : captures
    RUNS ||--o{ WORKTREES : isolates
    RUNS ||--o{ RECOVERY_ACTIONS : recovers
```

## 6. Scheduling and parallelism

```mermaid
flowchart TB
    store[(Ready tasks)] --> pick{Dependency-aware selection}
    pick -->|priority then age| claim[Atomic claim]
    claim --> serial[Serial execution]
    claim --> swarm[Swarm: isolated worktrees]
    claim --> arena[Arena: competing contestants]
    swarm --> verify[Verify each in isolation]
    arena --> judge[Judge and select winner]
    verify --> merge[Merge in order]
    judge --> merge
    merge --> unblock[Unblock dependents]
    unblock --> store
```

## 7. Decision flow (typed models)

```mermaid
flowchart LR
    req[Free-text request] --> enabled{Typed backend enabled?}
    enabled -->|yes| client[Decision client]
    client -->|confidence >= threshold| wt[Work type]
    client -->|low confidence or error| kw[Keyword router]
    enabled -->|no| kw
    kw --> wt
    wt --> wf[Workflow]
    wf --> plan[Plan and decompose]
```

## 8. Security and permissions

```mermaid
flowchart TB
    input[[External text: files, issues, PRs, web]] --> envelope[Untrusted envelope<br/>data, never instructions]
    envelope --> model[Agent / decision model]
    model --> proposal[Proposed action]
    proposal --> perm{Permission level}
    perm -->|read_only| observe[Allow observe]
    perm -->|safe| safe[Allow non-mutating]
    perm -->|standard| write[Allow writes]
    perm -->|autonomous| auto[Allow git/network]
    perm -->|dangerous| gate{Dangerous enabled?}
    gate -->|no| deny[Deny + audit event]
    gate -->|yes| careful{Careful / freeze}
    careful -->|blocked| deny
    careful -->|allowed| ledger[Execute + egress ledger]
```

## 9. Context assembly per iteration

```mermaid
flowchart LR
    budget{{Token budget}} --> builder[Context builder]
    principles[Stage principles] --> builder
    recall[Memory recall] --> builder
    journal[Recent journal] --> builder
    task[Current task + acceptance] --> builder
    progress[Progress] --> builder
    builder --> prompt[Fresh prompt]
    prompt --> session[Clean agent session]
    session --> result[Result + sigils]
    result --> persist[(Persist: attempts, evidence, journal, decisions)]
```

## 10. Distribution

```mermaid
flowchart LR
    subgraph source[Source]
        repo[(GitHub repository)]
    end
    subgraph binaries[Prebuilt binaries]
        rel[Release workflow<br/>tag vX.Y.Z]
        assets[linux · macOS · windows archives]
    end
    subgraph skillonly[Skills only]
        install["majstack install-skills --host ..."]
    end
    repo --> rel --> assets
    assets -->|install.sh / install.ps1| local[majstack · majstackd on PATH]
    repo --> install --> hosts[claude · codex · opencode · cursor · factory · kiro]
    local --> use["majstack init && majstack run"]
```
