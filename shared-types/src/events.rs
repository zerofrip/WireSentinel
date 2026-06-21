use crate::{
    AppRecord, BandwidthSnapshot, BehaviorAnomaly, BeaconingFinding, CasbFinding, ChainProfile,
    ComplianceCheckKind, ConditionalAccessResult, DetectionTrigger, DevicePosture, DeviceTrustRecord,
    DNSQueryLog, DlpIncident, DriverState, FirewallDecisionRecord, GatewayConnectionResult,
    IdentityProviderKind, IdentityThreat, Incident, IsolationSession, LateralMovementFinding,
    LeakIncident, MaliciousExecution, NetworkThreat, ObfuscationPreset, PerformanceSnapshot,
    PersistenceFinding, PlaybookExecution, PluginRecord, PrivacyAnalyticsSnapshot,
    PrivacyScoreSnapshot, ProcessAnomaly, ProxyProfile, ResponseActionResult, RiskScore,
    RouteStatisticsRecord, Rule, SecurityAuditEntry, SecurityFinding, SegmentPolicyResult,
    ShadowItRecord, SiemExportJob, SyncMode, TailnetProfile, TechniqueDetection, ThreatMatch,
    TrafficEvent, TrafficRoute, TrustScoreSnapshot, UserIdentity, WebAccessResult,
    AffectedAsset, CloudAttackPath, CnappSeverity, ComplianceControl, ComplianceScore,
    ContainerFinding, DependencyRecord, IacFinding, KubernetesFinding, PostureFinding,
    SbomDocument, SecretFinding, Vulnerability, WorkloadRecord,
    AiRecommendation, AiRiskScore, CopilotResponse, CorrelatedThreat,
    ExecutiveReport, InvestigationReport,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Strongly typed service events streamed over WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServiceEvent {
    VpnConnected {
        profile_id: Uuid,
        profile_name: String,
        timestamp: DateTime<Utc>,
    },
    VpnDisconnected {
        profile_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    TrafficObserved {
        event: TrafficEvent,
        timestamp: DateTime<Utc>,
    },
    TrafficBlocked {
        event: TrafficEvent,
        route: TrafficRoute,
        timestamp: DateTime<Utc>,
    },
    RuleCreated {
        rule: Rule,
        timestamp: DateTime<Utc>,
    },
    RuleUpdated {
        rule: Rule,
        timestamp: DateTime<Utc>,
    },
    RuleDeleted {
        rule_id: Uuid,
        timestamp: DateTime<Utc>,
    },
    DnsQueryObserved {
        log: DNSQueryLog,
        timestamp: DateTime<Utc>,
    },
    DnsQueryBlocked {
        log: DNSQueryLog,
        timestamp: DateTime<Utc>,
    },
    AppDiscovered {
        app: AppRecord,
        timestamp: DateTime<Utc>,
    },
    AppUpdated {
        app: AppRecord,
        timestamp: DateTime<Utc>,
    },
    BandwidthUpdated {
        snapshot: BandwidthSnapshot,
        timestamp: DateTime<Utc>,
    },
    SystemWarning {
        message: String,
        timestamp: DateTime<Utc>,
    },
    SystemError {
        message: String,
        timestamp: DateTime<Utc>,
    },
    ServiceStatus {
        status: crate::ServiceStatus,
        timestamp: DateTime<Utc>,
    },
    VpnError {
        profile_id: Uuid,
        message: String,
        timestamp: DateTime<Utc>,
    },
    FilterListUpdated {
        list_id: Uuid,
        name: String,
        entry_count: u32,
        timestamp: DateTime<Utc>,
    },
    FilterListFailed {
        list_id: Uuid,
        error: String,
        timestamp: DateTime<Utc>,
    },
    TrafficAllowed {
        event: TrafficEvent,
        timestamp: DateTime<Utc>,
    },
    FirewallDecision {
        decision: FirewallDecisionRecord,
        timestamp: DateTime<Utc>,
    },
    RouteUsageUpdated {
        stats: RouteStatisticsRecord,
        timestamp: DateTime<Utc>,
    },
    PolicyChanged {
        field: String,
        old_value: Option<String>,
        new_value: Option<String>,
        timestamp: DateTime<Utc>,
    },
    RouteChanged {
        app_id: Uuid,
        old_route: Option<TrafficRoute>,
        new_route: Option<TrafficRoute>,
        timestamp: DateTime<Utc>,
    },
    TransportStarted {
        transport_id: Uuid,
        name: String,
        timestamp: DateTime<Utc>,
    },
    TransportStopped {
        transport_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    TransportError {
        transport_id: Uuid,
        message: String,
        timestamp: DateTime<Utc>,
    },
    TransportMetricsUpdated {
        transport_id: Uuid,
        rx_bytes: u64,
        tx_bytes: u64,
        timestamp: DateTime<Utc>,
    },
    DnsProviderChanged {
        provider_id: Uuid,
        provider_name: String,
        timestamp: DateTime<Utc>,
    },
    DnsProviderFailed {
        provider_id: Uuid,
        provider_name: String,
        error: String,
        timestamp: DateTime<Utc>,
    },
    PrivacyScoreUpdated {
        snapshot: PrivacyScoreSnapshot,
        timestamp: DateTime<Utc>,
    },
    LeakDetected {
        incident: LeakIncident,
        timestamp: DateTime<Utc>,
    },
    RecoveryStarted {
        scope: String,
        timestamp: DateTime<Utc>,
    },
    RecoveryCompleted {
        restored_count: u32,
        timestamp: DateTime<Utc>,
    },
    RecoveryFailed {
        scope: String,
        error: String,
        timestamp: DateTime<Utc>,
    },
    PerformanceSnapshot {
        snapshot: PerformanceSnapshot,
        timestamp: DateTime<Utc>,
    },
    SecurityAudit {
        entry: SecurityAuditEntry,
        timestamp: DateTime<Utc>,
    },
    ValidationPassed {
        check_name: String,
        message: Option<String>,
        timestamp: DateTime<Utc>,
    },
    ValidationFailed {
        check_name: String,
        message: String,
        timestamp: DateTime<Utc>,
    },
    DriverStateChanged {
        state: DriverState,
        timestamp: DateTime<Utc>,
    },
    DriverRecovered {
        recovery_generation: u32,
        timestamp: DateTime<Utc>,
    },
    DriverRecoveryFailed {
        error: String,
        timestamp: DateTime<Utc>,
    },
    FaultInjected {
        scenario: String,
        timestamp: DateTime<Utc>,
    },
    RecoveryVerified {
        scenario: String,
        timestamp: DateTime<Utc>,
    },
    SecurityFindingRecorded {
        finding: SecurityFinding,
        timestamp: DateTime<Utc>,
    },
    TailnetJoined {
        profile_id: Uuid,
        hostname: Option<String>,
        timestamp: DateTime<Utc>,
    },
    TailnetLeft {
        profile_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    ExitNodeChanged {
        profile_id: Uuid,
        exit_node: Option<String>,
        timestamp: DateTime<Utc>,
    },
    TorStarted {
        profile_id: Uuid,
        timestamp: DateTime<Utc>,
    },
    TorStopped {
        profile_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    TorCircuitChanged {
        profile_id: Uuid,
        circuit_count: u32,
        timestamp: DateTime<Utc>,
    },
    BridgeConnected {
        bridge_id: Uuid,
        profile_id: Uuid,
        timestamp: DateTime<Utc>,
    },
    BridgeFailed {
        bridge_id: Uuid,
        error: String,
        timestamp: DateTime<Utc>,
    },
    PluginInstalled {
        plugin: PluginRecord,
        timestamp: DateTime<Utc>,
    },
    PluginLoaded {
        plugin_id: Uuid,
        name: String,
        timestamp: DateTime<Utc>,
    },
    PluginUnloaded {
        plugin_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    PluginFailed {
        plugin_id: Uuid,
        error: String,
        timestamp: DateTime<Utc>,
    },
    PluginSecurityViolation {
        plugin_id: Uuid,
        violation_type: String,
        detail: String,
        timestamp: DateTime<Utc>,
    },
    TailnetProfileUpdated {
        profile: TailnetProfile,
        timestamp: DateTime<Utc>,
    },
    TorBootstrapProgress {
        profile_id: Uuid,
        progress: u8,
        timestamp: DateTime<Utc>,
    },
    TransportChainUpdated {
        chain: ChainProfile,
        timestamp: DateTime<Utc>,
    },
    TransportChainStarted {
        chain_id: Uuid,
        name: String,
        timestamp: DateTime<Utc>,
    },
    TransportChainStopped {
        chain_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    ProxyProfileCreated {
        profile: ProxyProfile,
        timestamp: DateTime<Utc>,
    },
    ProxyProfileUpdated {
        profile: ProxyProfile,
        timestamp: DateTime<Utc>,
    },
    ProxyLatencyMeasured {
        profile_id: Uuid,
        latency_ms: u64,
        timestamp: DateTime<Utc>,
    },
    ProxyConnected {
        profile_id: Uuid,
        listen_port: u16,
        timestamp: DateTime<Utc>,
    },
    ProxyDisconnected {
        profile_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    ProxyFailed {
        profile_id: Uuid,
        error: String,
        timestamp: DateTime<Utc>,
    },
    ProxyChainStarted {
        chain_id: Uuid,
        name: String,
        timestamp: DateTime<Utc>,
    },
    ProxyChainStopped {
        chain_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    AgentEnrolled {
        agent_id: Uuid,
        name: String,
        timestamp: DateTime<Utc>,
    },
    AgentRevoked {
        agent_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    ObfuscationProfileApplied {
        chain_id: Uuid,
        profile_id: Uuid,
        preset: ObfuscationPreset,
        timestamp: DateTime<Utc>,
    },
    MixnetStarted {
        profile_id: Uuid,
        timestamp: DateTime<Utc>,
    },
    MixnetStopped {
        profile_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    MixnetFailed {
        profile_id: Uuid,
        error: String,
        timestamp: DateTime<Utc>,
    },
    GatewayChanged {
        profile_id: Uuid,
        gateway_id: String,
        timestamp: DateTime<Utc>,
    },
    CoverTrafficStarted {
        profile_id: Uuid,
        timestamp: DateTime<Utc>,
    },
    CoverTrafficStopped {
        profile_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    AnonymousChainStarted {
        chain_id: Uuid,
        name: String,
        timestamp: DateTime<Utc>,
    },
    AnonymousChainStopped {
        chain_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    PrivacyAnalyticsUpdated {
        snapshot: PrivacyAnalyticsSnapshot,
        timestamp: DateTime<Utc>,
    },
    MixnetSecurityViolation {
        profile_id: Uuid,
        violation_type: String,
        detail: String,
        timestamp: DateTime<Utc>,
    },
    ControllerRegistered {
        controller_id: Uuid,
        url: String,
        timestamp: DateTime<Utc>,
    },
    ControllerDisconnected {
        controller_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    ControllerSynced {
        controller_id: Uuid,
        sync_mode: SyncMode,
        timestamp: DateTime<Utc>,
    },
    UserAuthenticated {
        subject: String,
        email: Option<String>,
        timestamp: DateTime<Utc>,
    },
    UserProvisioned {
        user_id: Uuid,
        email: String,
        timestamp: DateTime<Utc>,
    },
    UserDeprovisioned {
        user_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    CompliancePassed {
        report_id: Uuid,
        check_kind: ComplianceCheckKind,
        timestamp: DateTime<Utc>,
    },
    ComplianceFailed {
        report_id: Uuid,
        check_kind: ComplianceCheckKind,
        detail: String,
        timestamp: DateTime<Utc>,
    },
    ComplianceWarning {
        report_id: Uuid,
        check_kind: ComplianceCheckKind,
        detail: String,
        timestamp: DateTime<Utc>,
    },
    CloudSecurityViolation {
        tenant_id: Uuid,
        violation_type: String,
        detail: String,
        timestamp: DateTime<Utc>,
    },
    QuotaExceeded {
        tenant_id: Uuid,
        quota: String,
        limit: u64,
        current: u64,
        timestamp: DateTime<Utc>,
    },
    KernelSecurityViolation {
        violation_type: String,
        detail: String,
        timestamp: DateTime<Utc>,
    },
    DriverIntegrityFailure {
        driver: String,
        detail: String,
        timestamp: DateTime<Utc>,
    },
    KatzenpostStarted {
        profile_id: Uuid,
        timestamp: DateTime<Utc>,
    },
    KatzenpostStopped {
        profile_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    KatzenpostFailed {
        profile_id: Uuid,
        error: String,
        timestamp: DateTime<Utc>,
    },
    LoopixStarted {
        profile_id: Uuid,
        timestamp: DateTime<Utc>,
    },
    LoopixStopped {
        profile_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    LoopixFailed {
        profile_id: Uuid,
        error: String,
        timestamp: DateTime<Utc>,
    },
    MixnetFederationUpdated {
        profile_id: Uuid,
        providers: Vec<String>,
        timestamp: DateTime<Utc>,
    },
    AdaptiveCoverUpdated {
        adaptive: bool,
        timestamp: DateTime<Utc>,
    },
    EntropyScoreUpdated {
        score: f64,
        anonymity_set_estimate: f64,
        timestamp: DateTime<Utc>,
    },
    DecoyRouteCreated {
        route_id: Uuid,
        target: String,
        timestamp: DateTime<Utc>,
    },
    DecoyRouteSimulated {
        route_id: Uuid,
        simulated_hops: u32,
        timestamp: DateTime<Utc>,
    },
    AnonymityAnalyticsUpdated {
        snapshot: PrivacyAnalyticsSnapshot,
        timestamp: DateTime<Utc>,
    },
    AnonymitySecurityViolation {
        profile_id: Uuid,
        violation_type: String,
        detail: String,
        timestamp: DateTime<Utc>,
    },
    NodeFailed {
        node_id: String,
        node_type: String,
        error: String,
        timestamp: DateTime<Utc>,
    },
    FailoverTriggered {
        scope: String,
        from_node: Option<String>,
        to_node: Option<String>,
        timestamp: DateTime<Utc>,
    },
    BillingSecurityViolation {
        tenant_id: Uuid,
        violation_type: String,
        detail: String,
        timestamp: DateTime<Utc>,
    },
    IdentityAuthenticated {
        user: UserIdentity,
        provider: IdentityProviderKind,
        timestamp: DateTime<Utc>,
    },
    IdentityFailed {
        subject: String,
        provider: IdentityProviderKind,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    IdentityProviderUpdated {
        provider: IdentityProviderKind,
        enabled: bool,
        timestamp: DateTime<Utc>,
    },
    DeviceTrustUpdated {
        record: DeviceTrustRecord,
        timestamp: DateTime<Utc>,
    },
    DevicePostureChanged {
        device_id: Uuid,
        posture: DevicePosture,
        timestamp: DateTime<Utc>,
    },
    ConditionalAccessEvaluated {
        result: ConditionalAccessResult,
        timestamp: DateTime<Utc>,
    },
    AccessDenied {
        subject_id: Uuid,
        resource_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    GatewayConnectionEstablished {
        result: GatewayConnectionResult,
        timestamp: DateTime<Utc>,
    },
    GatewayConnectionDenied {
        gateway_id: Uuid,
        subject_id: Uuid,
        resource_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    SegmentPolicyApplied {
        result: SegmentPolicyResult,
        timestamp: DateTime<Utc>,
    },
    SegmentPolicyDenied {
        segment_id: Uuid,
        subject_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    TrustScoreUpdated {
        snapshot: TrustScoreSnapshot,
        timestamp: DateTime<Utc>,
    },
    ZtnaSecurityViolation {
        violation_type: String,
        detail: String,
        timestamp: DateTime<Utc>,
    },
    IdentitySecurityViolation {
        violation_type: String,
        detail: String,
        timestamp: DateTime<Utc>,
    },
    UsageThresholdReached {
        tenant_id: Uuid,
        metric: String,
        threshold: f64,
        current: f64,
        timestamp: DateTime<Utc>,
    },
    WebAccessAllowed {
        result: WebAccessResult,
        timestamp: DateTime<Utc>,
    },
    WebAccessBlocked {
        result: WebAccessResult,
        timestamp: DateTime<Utc>,
    },
    WebAccessViolation {
        result: WebAccessResult,
        timestamp: DateTime<Utc>,
    },
    ShadowItDetected {
        record: ShadowItRecord,
        timestamp: DateTime<Utc>,
    },
    CasbViolation {
        finding: CasbFinding,
        timestamp: DateTime<Utc>,
    },
    DlpViolation {
        incident: DlpIncident,
        timestamp: DateTime<Utc>,
    },
    SensitiveDataDetected {
        incident: DlpIncident,
        timestamp: DateTime<Utc>,
    },
    IsolationStarted {
        session: IsolationSession,
        timestamp: DateTime<Utc>,
    },
    IsolationTerminated {
        session: IsolationSession,
        timestamp: DateTime<Utc>,
    },
    ThreatDetected {
        threat: ThreatMatch,
        timestamp: DateTime<Utc>,
    },
    ThreatBlocked {
        threat: ThreatMatch,
        timestamp: DateTime<Utc>,
    },
    AnomalyDetected {
        anomaly: BehaviorAnomaly,
        timestamp: DateTime<Utc>,
    },
    RiskElevated {
        score: RiskScore,
        previous_level: crate::RiskLevel,
        timestamp: DateTime<Utc>,
    },
    RiskScoreUpdated {
        score: RiskScore,
        timestamp: DateTime<Utc>,
    },
    RiskThresholdExceeded {
        score: RiskScore,
        threshold: u8,
        timestamp: DateTime<Utc>,
    },
    SiemExportStarted {
        job: SiemExportJob,
        timestamp: DateTime<Utc>,
    },
    SiemExportCompleted {
        job: SiemExportJob,
        timestamp: DateTime<Utc>,
    },
    SiemExportFailed {
        job: SiemExportJob,
        timestamp: DateTime<Utc>,
    },
    SseSecurityViolation {
        violation_type: String,
        detail: String,
        timestamp: DateTime<Utc>,
    },
    ProcessAnomalyDetected {
        anomaly: ProcessAnomaly,
        timestamp: DateTime<Utc>,
    },
    PersistenceDetected {
        finding: PersistenceFinding,
        timestamp: DateTime<Utc>,
    },
    MaliciousExecutionDetected {
        execution: MaliciousExecution,
        timestamp: DateTime<Utc>,
    },
    NetworkThreatDetected {
        threat: NetworkThreat,
        timestamp: DateTime<Utc>,
    },
    BeaconingDetected {
        finding: BeaconingFinding,
        timestamp: DateTime<Utc>,
    },
    LateralMovementDetected {
        finding: LateralMovementFinding,
        timestamp: DateTime<Utc>,
    },
    IdentityThreatDetected {
        threat: IdentityThreat,
        timestamp: DateTime<Utc>,
    },
    IdentityCompromiseSuspected {
        threat: IdentityThreat,
        timestamp: DateTime<Utc>,
    },
    DetectionTriggered {
        trigger: DetectionTrigger,
        timestamp: DateTime<Utc>,
    },
    IncidentCreated {
        incident: Incident,
        timestamp: DateTime<Utc>,
    },
    IncidentEscalated {
        incident: Incident,
        timestamp: DateTime<Utc>,
    },
    IncidentResolved {
        incident: Incident,
        timestamp: DateTime<Utc>,
    },
    PlaybookStarted {
        execution: PlaybookExecution,
        timestamp: DateTime<Utc>,
    },
    PlaybookCompleted {
        execution: PlaybookExecution,
        timestamp: DateTime<Utc>,
    },
    PlaybookFailed {
        execution: PlaybookExecution,
        timestamp: DateTime<Utc>,
    },
    TechniqueDetected {
        detection: TechniqueDetection,
        timestamp: DateTime<Utc>,
    },
    ResponseActionExecuted {
        result: ResponseActionResult,
        timestamp: DateTime<Utc>,
    },
    ResponseActionFailed {
        result: ResponseActionResult,
        timestamp: DateTime<Utc>,
    },
    XdrSecurityViolation {
        violation_type: String,
        detail: String,
        timestamp: DateTime<Utc>,
    },
    CloudMisconfigurationDetected {
        finding: PostureFinding,
        timestamp: DateTime<Utc>,
    },
    CloudRiskIncreased {
        tenant_id: Uuid,
        previous_score: f64,
        current_score: f64,
        timestamp: DateTime<Utc>,
    },
    CloudPolicyViolation {
        finding: PostureFinding,
        timestamp: DateTime<Utc>,
    },
    WorkloadThreatDetected {
        workload: WorkloadRecord,
        severity: CnappSeverity,
        timestamp: DateTime<Utc>,
    },
    WorkloadCompromised {
        workload: WorkloadRecord,
        timestamp: DateTime<Utc>,
    },
    KubernetesRiskDetected {
        finding: KubernetesFinding,
        timestamp: DateTime<Utc>,
    },
    ClusterCompromiseSuspected {
        finding: KubernetesFinding,
        timestamp: DateTime<Utc>,
    },
    ContainerRiskDetected {
        finding: ContainerFinding,
        timestamp: DateTime<Utc>,
    },
    ContainerThreatDetected {
        finding: ContainerFinding,
        timestamp: DateTime<Utc>,
    },
    IacFindingDetected {
        finding: IacFinding,
        timestamp: DateTime<Utc>,
    },
    SecretExposed {
        finding: SecretFinding,
        timestamp: DateTime<Utc>,
    },
    DependencyRiskDetected {
        dependency: DependencyRecord,
        severity: CnappSeverity,
        timestamp: DateTime<Utc>,
    },
    SupplyChainThreatDetected {
        dependency: DependencyRecord,
        timestamp: DateTime<Utc>,
    },
    SbomGenerated {
        document: SbomDocument,
        timestamp: DateTime<Utc>,
    },
    SbomImported {
        document: SbomDocument,
        timestamp: DateTime<Utc>,
    },
    CriticalVulnerabilityDetected {
        vulnerability: Vulnerability,
        asset: AffectedAsset,
        timestamp: DateTime<Utc>,
    },
    AttackPathDiscovered {
        path: CloudAttackPath,
        timestamp: DateTime<Utc>,
    },
    ComplianceViolation {
        control: ComplianceControl,
        severity: CnappSeverity,
        timestamp: DateTime<Utc>,
    },
    ComplianceScoreUpdated {
        score: ComplianceScore,
        timestamp: DateTime<Utc>,
    },
    CnappSecurityViolation {
        violation_type: String,
        detail: String,
        timestamp: DateTime<Utc>,
    },
    CopilotQueryExecuted {
        response: CopilotResponse,
        timestamp: DateTime<Utc>,
    },
    InvestigationCompleted {
        report: InvestigationReport,
        timestamp: DateTime<Utc>,
    },
    ThreatCorrelated {
        threat: CorrelatedThreat,
        timestamp: DateTime<Utc>,
    },
    AiRiskScoreUpdated {
        score: AiRiskScore,
        timestamp: DateTime<Utc>,
    },
    AiRecommendationGenerated {
        recommendation: AiRecommendation,
        timestamp: DateTime<Utc>,
    },
    ExecutiveReportGenerated {
        report: ExecutiveReport,
        timestamp: DateTime<Utc>,
    },
    AiSecurityViolation {
        violation_type: String,
        detail: String,
        timestamp: DateTime<Utc>,
    },
    PromptBlocked {
        tenant_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    ProviderAccessDenied {
        tenant_id: Uuid,
        provider: String,
        timestamp: DateTime<Utc>,
    },
}

impl ServiceEvent {
    pub fn now(event: ServiceEventInner) -> Self {
        event.with_timestamp(Utc::now())
    }
}

/// Helper to build events with explicit payloads.
pub enum ServiceEventInner {
    VpnConnected { profile_id: Uuid, profile_name: String },
    VpnDisconnected { profile_id: Uuid, reason: String },
    TrafficObserved { event: TrafficEvent },
    TrafficBlocked { event: TrafficEvent, route: TrafficRoute },
    RuleCreated { rule: Rule },
    RuleUpdated { rule: Rule },
    RuleDeleted { rule_id: Uuid },
    DnsQueryObserved { log: DNSQueryLog },
    DnsQueryBlocked { log: DNSQueryLog },
    AppDiscovered { app: AppRecord },
    AppUpdated { app: AppRecord },
    BandwidthUpdated { snapshot: BandwidthSnapshot },
    SystemWarning { message: String },
    SystemError { message: String },
    ServiceStatus { status: crate::ServiceStatus },
    VpnError { profile_id: Uuid, message: String },
    FilterListUpdated {
        list_id: Uuid,
        name: String,
        entry_count: u32,
    },
    FilterListFailed { list_id: Uuid, error: String },
    TrafficAllowed { event: TrafficEvent },
    FirewallDecision { decision: FirewallDecisionRecord },
    RouteUsageUpdated { stats: RouteStatisticsRecord },
    PolicyChanged {
        field: String,
        old_value: Option<String>,
        new_value: Option<String>,
    },
    RouteChanged {
        app_id: Uuid,
        old_route: Option<TrafficRoute>,
        new_route: Option<TrafficRoute>,
    },
    TransportStarted {
        transport_id: Uuid,
        name: String,
    },
    TransportStopped {
        transport_id: Uuid,
        reason: String,
    },
    TransportError {
        transport_id: Uuid,
        message: String,
    },
    TransportMetricsUpdated {
        transport_id: Uuid,
        rx_bytes: u64,
        tx_bytes: u64,
    },
    DnsProviderChanged {
        provider_id: Uuid,
        provider_name: String,
    },
    DnsProviderFailed {
        provider_id: Uuid,
        provider_name: String,
        error: String,
    },
    PrivacyScoreUpdated {
        snapshot: PrivacyScoreSnapshot,
    },
    LeakDetected {
        incident: LeakIncident,
    },
    RecoveryStarted {
        scope: String,
    },
    RecoveryCompleted {
        restored_count: u32,
    },
    RecoveryFailed {
        scope: String,
        error: String,
    },
    PerformanceSnapshot {
        snapshot: PerformanceSnapshot,
    },
    SecurityAudit {
        entry: SecurityAuditEntry,
    },
    ValidationPassed {
        check_name: String,
        message: Option<String>,
    },
    ValidationFailed {
        check_name: String,
        message: String,
    },
    DriverStateChanged {
        state: DriverState,
    },
    DriverRecovered {
        recovery_generation: u32,
    },
    DriverRecoveryFailed {
        error: String,
    },
    FaultInjected {
        scenario: String,
    },
    RecoveryVerified {
        scenario: String,
    },
    SecurityFindingRecorded {
        finding: SecurityFinding,
    },
    TailnetJoined {
        profile_id: Uuid,
        hostname: Option<String>,
    },
    TailnetLeft {
        profile_id: Uuid,
        reason: String,
    },
    ExitNodeChanged {
        profile_id: Uuid,
        exit_node: Option<String>,
    },
    TorStarted {
        profile_id: Uuid,
    },
    TorStopped {
        profile_id: Uuid,
        reason: String,
    },
    TorCircuitChanged {
        profile_id: Uuid,
        circuit_count: u32,
    },
    BridgeConnected {
        bridge_id: Uuid,
        profile_id: Uuid,
    },
    BridgeFailed {
        bridge_id: Uuid,
        error: String,
    },
    PluginInstalled { plugin: PluginRecord },
    PluginLoaded { plugin_id: Uuid, name: String },
    PluginUnloaded { plugin_id: Uuid, reason: String },
    PluginFailed { plugin_id: Uuid, error: String },
    PluginSecurityViolation {
        plugin_id: Uuid,
        violation_type: String,
        detail: String,
    },
    TailnetProfileUpdated { profile: TailnetProfile },
    TorBootstrapProgress { profile_id: Uuid, progress: u8 },
    TransportChainUpdated { chain: ChainProfile },
    TransportChainStarted { chain_id: Uuid, name: String },
    TransportChainStopped { chain_id: Uuid, reason: String },
    ProxyProfileCreated { profile: ProxyProfile },
    ProxyProfileUpdated { profile: ProxyProfile },
    ProxyLatencyMeasured { profile_id: Uuid, latency_ms: u64 },
    ProxyConnected { profile_id: Uuid, listen_port: u16 },
    ProxyDisconnected { profile_id: Uuid, reason: String },
    ProxyFailed { profile_id: Uuid, error: String },
    ProxyChainStarted { chain_id: Uuid, name: String },
    ProxyChainStopped { chain_id: Uuid, reason: String },
    AgentEnrolled { agent_id: Uuid, name: String },
    AgentRevoked { agent_id: Uuid, reason: String },
    ObfuscationProfileApplied {
        chain_id: Uuid,
        profile_id: Uuid,
        preset: ObfuscationPreset,
    },
    MixnetStarted { profile_id: Uuid },
    MixnetStopped { profile_id: Uuid, reason: String },
    MixnetFailed { profile_id: Uuid, error: String },
    GatewayChanged {
        profile_id: Uuid,
        gateway_id: String,
    },
    CoverTrafficStarted { profile_id: Uuid },
    CoverTrafficStopped { profile_id: Uuid, reason: String },
    AnonymousChainStarted { chain_id: Uuid, name: String },
    AnonymousChainStopped { chain_id: Uuid, reason: String },
    PrivacyAnalyticsUpdated {
        snapshot: PrivacyAnalyticsSnapshot,
    },
    MixnetSecurityViolation {
        profile_id: Uuid,
        violation_type: String,
        detail: String,
    },
    ControllerRegistered {
        controller_id: Uuid,
        url: String,
    },
    ControllerDisconnected {
        controller_id: Uuid,
        reason: String,
    },
    ControllerSynced {
        controller_id: Uuid,
        sync_mode: SyncMode,
    },
    UserAuthenticated {
        subject: String,
        email: Option<String>,
    },
    UserProvisioned {
        user_id: Uuid,
        email: String,
    },
    UserDeprovisioned {
        user_id: Uuid,
        reason: String,
    },
    CompliancePassed {
        report_id: Uuid,
        check_kind: ComplianceCheckKind,
    },
    ComplianceFailed {
        report_id: Uuid,
        check_kind: ComplianceCheckKind,
        detail: String,
    },
    ComplianceWarning {
        report_id: Uuid,
        check_kind: ComplianceCheckKind,
        detail: String,
    },
    CloudSecurityViolation {
        tenant_id: Uuid,
        violation_type: String,
        detail: String,
    },
    QuotaExceeded {
        tenant_id: Uuid,
        quota: String,
        limit: u64,
        current: u64,
    },
    KernelSecurityViolation {
        violation_type: String,
        detail: String,
    },
    DriverIntegrityFailure {
        driver: String,
        detail: String,
    },
    KatzenpostStarted { profile_id: Uuid },
    KatzenpostStopped { profile_id: Uuid, reason: String },
    KatzenpostFailed { profile_id: Uuid, error: String },
    LoopixStarted { profile_id: Uuid },
    LoopixStopped { profile_id: Uuid, reason: String },
    LoopixFailed { profile_id: Uuid, error: String },
    MixnetFederationUpdated {
        profile_id: Uuid,
        providers: Vec<String>,
    },
    AdaptiveCoverUpdated { adaptive: bool },
    EntropyScoreUpdated {
        score: f64,
        anonymity_set_estimate: f64,
    },
    DecoyRouteCreated {
        route_id: Uuid,
        target: String,
    },
    DecoyRouteSimulated {
        route_id: Uuid,
        simulated_hops: u32,
    },
    AnonymityAnalyticsUpdated {
        snapshot: PrivacyAnalyticsSnapshot,
    },
    AnonymitySecurityViolation {
        profile_id: Uuid,
        violation_type: String,
        detail: String,
    },
    NodeFailed {
        node_id: String,
        node_type: String,
        error: String,
    },
    FailoverTriggered {
        scope: String,
        from_node: Option<String>,
        to_node: Option<String>,
    },
    BillingSecurityViolation {
        tenant_id: Uuid,
        violation_type: String,
        detail: String,
    },
    IdentityAuthenticated {
        user: UserIdentity,
        provider: IdentityProviderKind,
    },
    IdentityFailed {
        subject: String,
        provider: IdentityProviderKind,
        reason: String,
    },
    IdentityProviderUpdated {
        provider: IdentityProviderKind,
        enabled: bool,
    },
    DeviceTrustUpdated { record: DeviceTrustRecord },
    DevicePostureChanged {
        device_id: Uuid,
        posture: DevicePosture,
    },
    ConditionalAccessEvaluated { result: ConditionalAccessResult },
    AccessDenied {
        subject_id: Uuid,
        resource_id: Uuid,
        reason: String,
    },
    GatewayConnectionEstablished { result: GatewayConnectionResult },
    GatewayConnectionDenied {
        gateway_id: Uuid,
        subject_id: Uuid,
        resource_id: Uuid,
        reason: String,
    },
    SegmentPolicyApplied { result: SegmentPolicyResult },
    SegmentPolicyDenied {
        segment_id: Uuid,
        subject_id: Uuid,
        reason: String,
    },
    TrustScoreUpdated { snapshot: TrustScoreSnapshot },
    ZtnaSecurityViolation {
        violation_type: String,
        detail: String,
    },
    IdentitySecurityViolation {
        violation_type: String,
        detail: String,
    },
    UsageThresholdReached {
        tenant_id: Uuid,
        metric: String,
        threshold: f64,
        current: f64,
    },
    WebAccessAllowed { result: WebAccessResult },
    WebAccessBlocked { result: WebAccessResult },
    WebAccessViolation { result: WebAccessResult },
    ShadowItDetected { record: ShadowItRecord },
    CasbViolation { finding: CasbFinding },
    DlpViolation { incident: DlpIncident },
    SensitiveDataDetected { incident: DlpIncident },
    IsolationStarted { session: IsolationSession },
    IsolationTerminated { session: IsolationSession },
    ThreatDetected { threat: ThreatMatch },
    ThreatBlocked { threat: ThreatMatch },
    AnomalyDetected { anomaly: BehaviorAnomaly },
    RiskElevated {
        score: RiskScore,
        previous_level: crate::RiskLevel,
    },
    RiskScoreUpdated { score: RiskScore },
    RiskThresholdExceeded {
        score: RiskScore,
        threshold: u8,
    },
    SiemExportStarted { job: SiemExportJob },
    SiemExportCompleted { job: SiemExportJob },
    SiemExportFailed { job: SiemExportJob },
    SseSecurityViolation {
        violation_type: String,
        detail: String,
    },
    ProcessAnomalyDetected { anomaly: ProcessAnomaly },
    PersistenceDetected { finding: PersistenceFinding },
    MaliciousExecutionDetected { execution: MaliciousExecution },
    NetworkThreatDetected { threat: NetworkThreat },
    BeaconingDetected { finding: BeaconingFinding },
    LateralMovementDetected { finding: LateralMovementFinding },
    IdentityThreatDetected { threat: IdentityThreat },
    IdentityCompromiseSuspected { threat: IdentityThreat },
    DetectionTriggered { trigger: DetectionTrigger },
    IncidentCreated { incident: Incident },
    IncidentEscalated { incident: Incident },
    IncidentResolved { incident: Incident },
    PlaybookStarted { execution: PlaybookExecution },
    PlaybookCompleted { execution: PlaybookExecution },
    PlaybookFailed { execution: PlaybookExecution },
    TechniqueDetected { detection: TechniqueDetection },
    ResponseActionExecuted { result: ResponseActionResult },
    ResponseActionFailed { result: ResponseActionResult },
    XdrSecurityViolation {
        violation_type: String,
        detail: String,
    },
    CloudMisconfigurationDetected { finding: PostureFinding },
    CloudRiskIncreased {
        tenant_id: Uuid,
        previous_score: f64,
        current_score: f64,
    },
    CloudPolicyViolation { finding: PostureFinding },
    WorkloadThreatDetected {
        workload: WorkloadRecord,
        severity: CnappSeverity,
    },
    WorkloadCompromised { workload: WorkloadRecord },
    KubernetesRiskDetected { finding: KubernetesFinding },
    ClusterCompromiseSuspected { finding: KubernetesFinding },
    ContainerRiskDetected { finding: ContainerFinding },
    ContainerThreatDetected { finding: ContainerFinding },
    IacFindingDetected { finding: IacFinding },
    SecretExposed { finding: SecretFinding },
    DependencyRiskDetected {
        dependency: DependencyRecord,
        severity: CnappSeverity,
    },
    SupplyChainThreatDetected { dependency: DependencyRecord },
    SbomGenerated { document: SbomDocument },
    SbomImported { document: SbomDocument },
    CriticalVulnerabilityDetected {
        vulnerability: Vulnerability,
        asset: AffectedAsset,
    },
    AttackPathDiscovered { path: CloudAttackPath },
    ComplianceViolation {
        control: ComplianceControl,
        severity: CnappSeverity,
    },
    ComplianceScoreUpdated { score: ComplianceScore },
    CnappSecurityViolation {
        violation_type: String,
        detail: String,
    },
    CopilotQueryExecuted { response: CopilotResponse },
    InvestigationCompleted { report: InvestigationReport },
    ThreatCorrelated { threat: CorrelatedThreat },
    AiRiskScoreUpdated { score: AiRiskScore },
    AiRecommendationGenerated { recommendation: AiRecommendation },
    ExecutiveReportGenerated { report: ExecutiveReport },
    AiSecurityViolation {
        violation_type: String,
        detail: String,
    },
    PromptBlocked {
        tenant_id: Uuid,
        reason: String,
    },
    ProviderAccessDenied {
        tenant_id: Uuid,
        provider: String,
    },
}

impl ServiceEventInner {
    pub fn with_timestamp(self, timestamp: DateTime<Utc>) -> ServiceEvent {
        match self {
            Self::VpnConnected { profile_id, profile_name } => ServiceEvent::VpnConnected {
                profile_id,
                profile_name,
                timestamp,
            },
            Self::VpnDisconnected { profile_id, reason } => ServiceEvent::VpnDisconnected {
                profile_id,
                reason,
                timestamp,
            },
            Self::TrafficObserved { event } => ServiceEvent::TrafficObserved { event, timestamp },
            Self::TrafficBlocked { event, route } => {
                ServiceEvent::TrafficBlocked { event, route, timestamp }
            }
            Self::RuleCreated { rule } => ServiceEvent::RuleCreated { rule, timestamp },
            Self::RuleUpdated { rule } => ServiceEvent::RuleUpdated { rule, timestamp },
            Self::RuleDeleted { rule_id } => ServiceEvent::RuleDeleted { rule_id, timestamp },
            Self::DnsQueryObserved { log } => ServiceEvent::DnsQueryObserved { log, timestamp },
            Self::DnsQueryBlocked { log } => ServiceEvent::DnsQueryBlocked { log, timestamp },
            Self::AppDiscovered { app } => ServiceEvent::AppDiscovered { app, timestamp },
            Self::AppUpdated { app } => ServiceEvent::AppUpdated { app, timestamp },
            Self::BandwidthUpdated { snapshot } => {
                ServiceEvent::BandwidthUpdated { snapshot, timestamp }
            }
            Self::SystemWarning { message } => ServiceEvent::SystemWarning { message, timestamp },
            Self::SystemError { message } => ServiceEvent::SystemError { message, timestamp },
            Self::ServiceStatus { status } => ServiceEvent::ServiceStatus { status, timestamp },
            Self::VpnError { profile_id, message } => {
                ServiceEvent::VpnError {
                    profile_id,
                    message,
                    timestamp,
                }
            }
            Self::FilterListUpdated {
                list_id,
                name,
                entry_count,
            } => ServiceEvent::FilterListUpdated {
                list_id,
                name,
                entry_count,
                timestamp,
            },
            Self::FilterListFailed { list_id, error } => ServiceEvent::FilterListFailed {
                list_id,
                error,
                timestamp,
            },
            Self::TrafficAllowed { event } => ServiceEvent::TrafficAllowed { event, timestamp },
            Self::FirewallDecision { decision } => {
                ServiceEvent::FirewallDecision { decision, timestamp }
            }
            Self::RouteUsageUpdated { stats } => {
                ServiceEvent::RouteUsageUpdated { stats, timestamp }
            }
            Self::PolicyChanged {
                field,
                old_value,
                new_value,
            } => ServiceEvent::PolicyChanged {
                field,
                old_value,
                new_value,
                timestamp,
            },
            Self::RouteChanged {
                app_id,
                old_route,
                new_route,
            } => ServiceEvent::RouteChanged {
                app_id,
                old_route,
                new_route,
                timestamp,
            },
            Self::TransportStarted {
                transport_id,
                name,
            } => ServiceEvent::TransportStarted {
                transport_id,
                name,
                timestamp,
            },
            Self::TransportStopped {
                transport_id,
                reason,
            } => ServiceEvent::TransportStopped {
                transport_id,
                reason,
                timestamp,
            },
            Self::TransportError {
                transport_id,
                message,
            } => ServiceEvent::TransportError {
                transport_id,
                message,
                timestamp,
            },
            Self::TransportMetricsUpdated {
                transport_id,
                rx_bytes,
                tx_bytes,
            } => ServiceEvent::TransportMetricsUpdated {
                transport_id,
                rx_bytes,
                tx_bytes,
                timestamp,
            },
            Self::DnsProviderChanged {
                provider_id,
                provider_name,
            } => ServiceEvent::DnsProviderChanged {
                provider_id,
                provider_name,
                timestamp,
            },
            Self::DnsProviderFailed {
                provider_id,
                provider_name,
                error,
            } => ServiceEvent::DnsProviderFailed {
                provider_id,
                provider_name,
                error,
                timestamp,
            },
            Self::PrivacyScoreUpdated { snapshot } => {
                ServiceEvent::PrivacyScoreUpdated { snapshot, timestamp }
            }
            Self::LeakDetected { incident } => ServiceEvent::LeakDetected {
                incident,
                timestamp,
            },
            Self::RecoveryStarted { scope } => ServiceEvent::RecoveryStarted { scope, timestamp },
            Self::RecoveryCompleted { restored_count } => ServiceEvent::RecoveryCompleted {
                restored_count,
                timestamp,
            },
            Self::RecoveryFailed { scope, error } => ServiceEvent::RecoveryFailed {
                scope,
                error,
                timestamp,
            },
            Self::PerformanceSnapshot { snapshot } => ServiceEvent::PerformanceSnapshot {
                snapshot,
                timestamp,
            },
            Self::SecurityAudit { entry } => ServiceEvent::SecurityAudit { entry, timestamp },
            Self::ValidationPassed { check_name, message } => ServiceEvent::ValidationPassed {
                check_name,
                message,
                timestamp,
            },
            Self::ValidationFailed { check_name, message } => ServiceEvent::ValidationFailed {
                check_name,
                message,
                timestamp,
            },
            Self::DriverStateChanged { state } => ServiceEvent::DriverStateChanged {
                state,
                timestamp,
            },
            Self::DriverRecovered { recovery_generation } => ServiceEvent::DriverRecovered {
                recovery_generation,
                timestamp,
            },
            Self::DriverRecoveryFailed { error } => ServiceEvent::DriverRecoveryFailed {
                error,
                timestamp,
            },
            Self::FaultInjected { scenario } => ServiceEvent::FaultInjected {
                scenario,
                timestamp,
            },
            Self::RecoveryVerified { scenario } => ServiceEvent::RecoveryVerified {
                scenario,
                timestamp,
            },
            Self::SecurityFindingRecorded { finding } => ServiceEvent::SecurityFindingRecorded {
                finding,
                timestamp,
            },
            Self::TailnetJoined {
                profile_id,
                hostname,
            } => ServiceEvent::TailnetJoined {
                profile_id,
                hostname,
                timestamp,
            },
            Self::TailnetLeft {
                profile_id,
                reason,
            } => ServiceEvent::TailnetLeft {
                profile_id,
                reason,
                timestamp,
            },
            Self::ExitNodeChanged {
                profile_id,
                exit_node,
            } => ServiceEvent::ExitNodeChanged {
                profile_id,
                exit_node,
                timestamp,
            },
            Self::TorStarted { profile_id } => ServiceEvent::TorStarted {
                profile_id,
                timestamp,
            },
            Self::TorStopped {
                profile_id,
                reason,
            } => ServiceEvent::TorStopped {
                profile_id,
                reason,
                timestamp,
            },
            Self::TorCircuitChanged {
                profile_id,
                circuit_count,
            } => ServiceEvent::TorCircuitChanged {
                profile_id,
                circuit_count,
                timestamp,
            },
            Self::BridgeConnected {
                bridge_id,
                profile_id,
            } => ServiceEvent::BridgeConnected {
                bridge_id,
                profile_id,
                timestamp,
            },
            Self::BridgeFailed { bridge_id, error } => ServiceEvent::BridgeFailed {
                bridge_id,
                error,
                timestamp,
            },
            Self::PluginInstalled { plugin } => ServiceEvent::PluginInstalled {
                plugin,
                timestamp,
            },
            Self::PluginLoaded { plugin_id, name } => ServiceEvent::PluginLoaded {
                plugin_id,
                name,
                timestamp,
            },
            Self::PluginUnloaded { plugin_id, reason } => ServiceEvent::PluginUnloaded {
                plugin_id,
                reason,
                timestamp,
            },
            Self::PluginFailed { plugin_id, error } => ServiceEvent::PluginFailed {
                plugin_id,
                error,
                timestamp,
            },
            Self::PluginSecurityViolation {
                plugin_id,
                violation_type,
                detail,
            } => ServiceEvent::PluginSecurityViolation {
                plugin_id,
                violation_type,
                detail,
                timestamp,
            },
            Self::TailnetProfileUpdated { profile } => ServiceEvent::TailnetProfileUpdated {
                profile,
                timestamp,
            },
            Self::TorBootstrapProgress { profile_id, progress } => {
                ServiceEvent::TorBootstrapProgress {
                    profile_id,
                    progress,
                    timestamp,
                }
            }
            Self::TransportChainUpdated { chain } => ServiceEvent::TransportChainUpdated {
                chain,
                timestamp,
            },
            Self::TransportChainStarted { chain_id, name } => ServiceEvent::TransportChainStarted {
                chain_id,
                name,
                timestamp,
            },
            Self::TransportChainStopped { chain_id, reason } => ServiceEvent::TransportChainStopped {
                chain_id,
                reason,
                timestamp,
            },
            Self::ProxyProfileCreated { profile } => ServiceEvent::ProxyProfileCreated {
                profile,
                timestamp,
            },
            Self::ProxyProfileUpdated { profile } => ServiceEvent::ProxyProfileUpdated {
                profile,
                timestamp,
            },
            Self::ProxyLatencyMeasured {
                profile_id,
                latency_ms,
            } => ServiceEvent::ProxyLatencyMeasured {
                profile_id,
                latency_ms,
                timestamp,
            },
            Self::ProxyConnected {
                profile_id,
                listen_port,
            } => ServiceEvent::ProxyConnected {
                profile_id,
                listen_port,
                timestamp,
            },
            Self::ProxyDisconnected { profile_id, reason } => {
                ServiceEvent::ProxyDisconnected {
                    profile_id,
                    reason,
                    timestamp,
                }
            }
            Self::ProxyFailed { profile_id, error } => ServiceEvent::ProxyFailed {
                profile_id,
                error,
                timestamp,
            },
            Self::ProxyChainStarted { chain_id, name } => ServiceEvent::ProxyChainStarted {
                chain_id,
                name,
                timestamp,
            },
            Self::ProxyChainStopped { chain_id, reason } => ServiceEvent::ProxyChainStopped {
                chain_id,
                reason,
                timestamp,
            },
            Self::AgentEnrolled { agent_id, name } => ServiceEvent::AgentEnrolled {
                agent_id,
                name,
                timestamp,
            },
            Self::AgentRevoked { agent_id, reason } => ServiceEvent::AgentRevoked {
                agent_id,
                reason,
                timestamp,
            },
            Self::ObfuscationProfileApplied {
                chain_id,
                profile_id,
                preset,
            } => ServiceEvent::ObfuscationProfileApplied {
                chain_id,
                profile_id,
                preset,
                timestamp,
            },
            Self::MixnetStarted { profile_id } => ServiceEvent::MixnetStarted {
                profile_id,
                timestamp,
            },
            Self::MixnetStopped { profile_id, reason } => ServiceEvent::MixnetStopped {
                profile_id,
                reason,
                timestamp,
            },
            Self::MixnetFailed { profile_id, error } => ServiceEvent::MixnetFailed {
                profile_id,
                error,
                timestamp,
            },
            Self::GatewayChanged {
                profile_id,
                gateway_id,
            } => ServiceEvent::GatewayChanged {
                profile_id,
                gateway_id,
                timestamp,
            },
            Self::CoverTrafficStarted { profile_id } => ServiceEvent::CoverTrafficStarted {
                profile_id,
                timestamp,
            },
            Self::CoverTrafficStopped { profile_id, reason } => {
                ServiceEvent::CoverTrafficStopped {
                    profile_id,
                    reason,
                    timestamp,
                }
            }
            Self::AnonymousChainStarted { chain_id, name } => {
                ServiceEvent::AnonymousChainStarted {
                    chain_id,
                    name,
                    timestamp,
                }
            }
            Self::AnonymousChainStopped { chain_id, reason } => {
                ServiceEvent::AnonymousChainStopped {
                    chain_id,
                    reason,
                    timestamp,
                }
            }
            Self::PrivacyAnalyticsUpdated { snapshot } => ServiceEvent::PrivacyAnalyticsUpdated {
                snapshot,
                timestamp,
            },
            Self::MixnetSecurityViolation {
                profile_id,
                violation_type,
                detail,
            } => ServiceEvent::MixnetSecurityViolation {
                profile_id,
                violation_type,
                detail,
                timestamp,
            },
            Self::ControllerRegistered {
                controller_id,
                url,
            } => ServiceEvent::ControllerRegistered {
                controller_id,
                url,
                timestamp,
            },
            Self::ControllerDisconnected {
                controller_id,
                reason,
            } => ServiceEvent::ControllerDisconnected {
                controller_id,
                reason,
                timestamp,
            },
            Self::ControllerSynced {
                controller_id,
                sync_mode,
            } => ServiceEvent::ControllerSynced {
                controller_id,
                sync_mode,
                timestamp,
            },
            Self::UserAuthenticated { subject, email } => ServiceEvent::UserAuthenticated {
                subject,
                email,
                timestamp,
            },
            Self::UserProvisioned { user_id, email } => ServiceEvent::UserProvisioned {
                user_id,
                email,
                timestamp,
            },
            Self::UserDeprovisioned { user_id, reason } => ServiceEvent::UserDeprovisioned {
                user_id,
                reason,
                timestamp,
            },
            Self::CompliancePassed { report_id, check_kind } => ServiceEvent::CompliancePassed {
                report_id,
                check_kind,
                timestamp,
            },
            Self::ComplianceFailed {
                report_id,
                check_kind,
                detail,
            } => ServiceEvent::ComplianceFailed {
                report_id,
                check_kind,
                detail,
                timestamp,
            },
            Self::ComplianceWarning {
                report_id,
                check_kind,
                detail,
            } => ServiceEvent::ComplianceWarning {
                report_id,
                check_kind,
                detail,
                timestamp,
            },
            Self::CloudSecurityViolation {
                tenant_id,
                violation_type,
                detail,
            } => ServiceEvent::CloudSecurityViolation {
                tenant_id,
                violation_type,
                detail,
                timestamp,
            },
            Self::QuotaExceeded {
                tenant_id,
                quota,
                limit,
                current,
            } => ServiceEvent::QuotaExceeded {
                tenant_id,
                quota,
                limit,
                current,
                timestamp,
            },
            Self::KernelSecurityViolation {
                violation_type,
                detail,
            } => ServiceEvent::KernelSecurityViolation {
                violation_type,
                detail,
                timestamp,
            },
            Self::DriverIntegrityFailure { driver, detail } => {
                ServiceEvent::DriverIntegrityFailure {
                    driver,
                    detail,
                    timestamp,
                }
            }
            Self::KatzenpostStarted { profile_id } => ServiceEvent::KatzenpostStarted {
                profile_id,
                timestamp,
            },
            Self::KatzenpostStopped { profile_id, reason } => ServiceEvent::KatzenpostStopped {
                profile_id,
                reason,
                timestamp,
            },
            Self::KatzenpostFailed { profile_id, error } => ServiceEvent::KatzenpostFailed {
                profile_id,
                error,
                timestamp,
            },
            Self::LoopixStarted { profile_id } => ServiceEvent::LoopixStarted {
                profile_id,
                timestamp,
            },
            Self::LoopixStopped { profile_id, reason } => ServiceEvent::LoopixStopped {
                profile_id,
                reason,
                timestamp,
            },
            Self::LoopixFailed { profile_id, error } => ServiceEvent::LoopixFailed {
                profile_id,
                error,
                timestamp,
            },
            Self::MixnetFederationUpdated {
                profile_id,
                providers,
            } => ServiceEvent::MixnetFederationUpdated {
                profile_id,
                providers,
                timestamp,
            },
            Self::AdaptiveCoverUpdated { adaptive } => ServiceEvent::AdaptiveCoverUpdated {
                adaptive,
                timestamp,
            },
            Self::EntropyScoreUpdated {
                score,
                anonymity_set_estimate,
            } => ServiceEvent::EntropyScoreUpdated {
                score,
                anonymity_set_estimate,
                timestamp,
            },
            Self::DecoyRouteCreated { route_id, target } => ServiceEvent::DecoyRouteCreated {
                route_id,
                target,
                timestamp,
            },
            Self::DecoyRouteSimulated {
                route_id,
                simulated_hops,
            } => ServiceEvent::DecoyRouteSimulated {
                route_id,
                simulated_hops,
                timestamp,
            },
            Self::AnonymityAnalyticsUpdated { snapshot } => {
                ServiceEvent::AnonymityAnalyticsUpdated {
                    snapshot,
                    timestamp,
                }
            }
            Self::AnonymitySecurityViolation {
                profile_id,
                violation_type,
                detail,
            } => ServiceEvent::AnonymitySecurityViolation {
                profile_id,
                violation_type,
                detail,
                timestamp,
            },
            Self::NodeFailed {
                node_id,
                node_type,
                error,
            } => ServiceEvent::NodeFailed {
                node_id,
                node_type,
                error,
                timestamp,
            },
            Self::FailoverTriggered {
                scope,
                from_node,
                to_node,
            } => ServiceEvent::FailoverTriggered {
                scope,
                from_node,
                to_node,
                timestamp,
            },
            Self::BillingSecurityViolation {
                tenant_id,
                violation_type,
                detail,
            } => ServiceEvent::BillingSecurityViolation {
                tenant_id,
                violation_type,
                detail,
                timestamp,
            },
            Self::IdentityAuthenticated { user, provider } => ServiceEvent::IdentityAuthenticated {
                user,
                provider,
                timestamp,
            },
            Self::IdentityFailed {
                subject,
                provider,
                reason,
            } => ServiceEvent::IdentityFailed {
                subject,
                provider,
                reason,
                timestamp,
            },
            Self::IdentityProviderUpdated { provider, enabled } => {
                ServiceEvent::IdentityProviderUpdated {
                    provider,
                    enabled,
                    timestamp,
                }
            }
            Self::DeviceTrustUpdated { record } => {
                ServiceEvent::DeviceTrustUpdated { record, timestamp }
            }
            Self::DevicePostureChanged {
                device_id,
                posture,
            } => ServiceEvent::DevicePostureChanged {
                device_id,
                posture,
                timestamp,
            },
            Self::ConditionalAccessEvaluated { result } => {
                ServiceEvent::ConditionalAccessEvaluated { result, timestamp }
            }
            Self::AccessDenied {
                subject_id,
                resource_id,
                reason,
            } => ServiceEvent::AccessDenied {
                subject_id,
                resource_id,
                reason,
                timestamp,
            },
            Self::GatewayConnectionEstablished { result } => {
                ServiceEvent::GatewayConnectionEstablished { result, timestamp }
            }
            Self::GatewayConnectionDenied {
                gateway_id,
                subject_id,
                resource_id,
                reason,
            } => ServiceEvent::GatewayConnectionDenied {
                gateway_id,
                subject_id,
                resource_id,
                reason,
                timestamp,
            },
            Self::SegmentPolicyApplied { result } => {
                ServiceEvent::SegmentPolicyApplied { result, timestamp }
            }
            Self::SegmentPolicyDenied {
                segment_id,
                subject_id,
                reason,
            } => ServiceEvent::SegmentPolicyDenied {
                segment_id,
                subject_id,
                reason,
                timestamp,
            },
            Self::TrustScoreUpdated { snapshot } => {
                ServiceEvent::TrustScoreUpdated { snapshot, timestamp }
            }
            Self::ZtnaSecurityViolation {
                violation_type,
                detail,
            } => ServiceEvent::ZtnaSecurityViolation {
                violation_type,
                detail,
                timestamp,
            },
            Self::IdentitySecurityViolation {
                violation_type,
                detail,
            } => ServiceEvent::IdentitySecurityViolation {
                violation_type,
                detail,
                timestamp,
            },
            Self::UsageThresholdReached {
                tenant_id,
                metric,
                threshold,
                current,
            } => ServiceEvent::UsageThresholdReached {
                tenant_id,
                metric,
                threshold,
                current,
                timestamp,
            },
            Self::WebAccessAllowed { result } => ServiceEvent::WebAccessAllowed { result, timestamp },
            Self::WebAccessBlocked { result } => ServiceEvent::WebAccessBlocked { result, timestamp },
            Self::WebAccessViolation { result } => {
                ServiceEvent::WebAccessViolation { result, timestamp }
            }
            Self::ShadowItDetected { record } => ServiceEvent::ShadowItDetected { record, timestamp },
            Self::CasbViolation { finding } => ServiceEvent::CasbViolation { finding, timestamp },
            Self::DlpViolation { incident } => ServiceEvent::DlpViolation { incident, timestamp },
            Self::SensitiveDataDetected { incident } => {
                ServiceEvent::SensitiveDataDetected { incident, timestamp }
            }
            Self::IsolationStarted { session } => ServiceEvent::IsolationStarted { session, timestamp },
            Self::IsolationTerminated { session } => {
                ServiceEvent::IsolationTerminated { session, timestamp }
            }
            Self::ThreatDetected { threat } => ServiceEvent::ThreatDetected { threat, timestamp },
            Self::ThreatBlocked { threat } => ServiceEvent::ThreatBlocked { threat, timestamp },
            Self::AnomalyDetected { anomaly } => ServiceEvent::AnomalyDetected { anomaly, timestamp },
            Self::RiskElevated {
                score,
                previous_level,
            } => ServiceEvent::RiskElevated {
                score,
                previous_level,
                timestamp,
            },
            Self::RiskScoreUpdated { score } => ServiceEvent::RiskScoreUpdated { score, timestamp },
            Self::RiskThresholdExceeded { score, threshold } => ServiceEvent::RiskThresholdExceeded {
                score,
                threshold,
                timestamp,
            },
            Self::SiemExportStarted { job } => ServiceEvent::SiemExportStarted { job, timestamp },
            Self::SiemExportCompleted { job } => ServiceEvent::SiemExportCompleted { job, timestamp },
            Self::SiemExportFailed { job } => ServiceEvent::SiemExportFailed { job, timestamp },
            Self::SseSecurityViolation {
                violation_type,
                detail,
            } => ServiceEvent::SseSecurityViolation {
                violation_type,
                detail,
                timestamp,
            },
            Self::ProcessAnomalyDetected { anomaly } => {
                ServiceEvent::ProcessAnomalyDetected { anomaly, timestamp }
            }
            Self::PersistenceDetected { finding } => {
                ServiceEvent::PersistenceDetected { finding, timestamp }
            }
            Self::MaliciousExecutionDetected { execution } => {
                ServiceEvent::MaliciousExecutionDetected { execution, timestamp }
            }
            Self::NetworkThreatDetected { threat } => {
                ServiceEvent::NetworkThreatDetected { threat, timestamp }
            }
            Self::BeaconingDetected { finding } => {
                ServiceEvent::BeaconingDetected { finding, timestamp }
            }
            Self::LateralMovementDetected { finding } => {
                ServiceEvent::LateralMovementDetected { finding, timestamp }
            }
            Self::IdentityThreatDetected { threat } => {
                ServiceEvent::IdentityThreatDetected { threat, timestamp }
            }
            Self::IdentityCompromiseSuspected { threat } => {
                ServiceEvent::IdentityCompromiseSuspected { threat, timestamp }
            }
            Self::DetectionTriggered { trigger } => {
                ServiceEvent::DetectionTriggered { trigger, timestamp }
            }
            Self::IncidentCreated { incident } => {
                ServiceEvent::IncidentCreated { incident, timestamp }
            }
            Self::IncidentEscalated { incident } => {
                ServiceEvent::IncidentEscalated { incident, timestamp }
            }
            Self::IncidentResolved { incident } => {
                ServiceEvent::IncidentResolved { incident, timestamp }
            }
            Self::PlaybookStarted { execution } => {
                ServiceEvent::PlaybookStarted { execution, timestamp }
            }
            Self::PlaybookCompleted { execution } => {
                ServiceEvent::PlaybookCompleted { execution, timestamp }
            }
            Self::PlaybookFailed { execution } => {
                ServiceEvent::PlaybookFailed { execution, timestamp }
            }
            Self::TechniqueDetected { detection } => {
                ServiceEvent::TechniqueDetected { detection, timestamp }
            }
            Self::ResponseActionExecuted { result } => {
                ServiceEvent::ResponseActionExecuted { result, timestamp }
            }
            Self::ResponseActionFailed { result } => {
                ServiceEvent::ResponseActionFailed { result, timestamp }
            }
            Self::XdrSecurityViolation {
                violation_type,
                detail,
            } => ServiceEvent::XdrSecurityViolation {
                violation_type,
                detail,
                timestamp,
            },
            Self::CloudMisconfigurationDetected { finding } => {
                ServiceEvent::CloudMisconfigurationDetected { finding, timestamp }
            }
            Self::CloudRiskIncreased {
                tenant_id,
                previous_score,
                current_score,
            } => ServiceEvent::CloudRiskIncreased {
                tenant_id,
                previous_score,
                current_score,
                timestamp,
            },
            Self::CloudPolicyViolation { finding } => {
                ServiceEvent::CloudPolicyViolation { finding, timestamp }
            }
            Self::WorkloadThreatDetected { workload, severity } => {
                ServiceEvent::WorkloadThreatDetected {
                    workload,
                    severity,
                    timestamp,
                }
            }
            Self::WorkloadCompromised { workload } => {
                ServiceEvent::WorkloadCompromised { workload, timestamp }
            }
            Self::KubernetesRiskDetected { finding } => {
                ServiceEvent::KubernetesRiskDetected { finding, timestamp }
            }
            Self::ClusterCompromiseSuspected { finding } => {
                ServiceEvent::ClusterCompromiseSuspected { finding, timestamp }
            }
            Self::ContainerRiskDetected { finding } => {
                ServiceEvent::ContainerRiskDetected { finding, timestamp }
            }
            Self::ContainerThreatDetected { finding } => {
                ServiceEvent::ContainerThreatDetected { finding, timestamp }
            }
            Self::IacFindingDetected { finding } => {
                ServiceEvent::IacFindingDetected { finding, timestamp }
            }
            Self::SecretExposed { finding } => {
                ServiceEvent::SecretExposed { finding, timestamp }
            }
            Self::DependencyRiskDetected { dependency, severity } => {
                ServiceEvent::DependencyRiskDetected {
                    dependency,
                    severity,
                    timestamp,
                }
            }
            Self::SupplyChainThreatDetected { dependency } => {
                ServiceEvent::SupplyChainThreatDetected { dependency, timestamp }
            }
            Self::SbomGenerated { document } => {
                ServiceEvent::SbomGenerated { document, timestamp }
            }
            Self::SbomImported { document } => {
                ServiceEvent::SbomImported { document, timestamp }
            }
            Self::CriticalVulnerabilityDetected { vulnerability, asset } => {
                ServiceEvent::CriticalVulnerabilityDetected {
                    vulnerability,
                    asset,
                    timestamp,
                }
            }
            Self::AttackPathDiscovered { path } => {
                ServiceEvent::AttackPathDiscovered { path, timestamp }
            }
            Self::ComplianceViolation { control, severity } => {
                ServiceEvent::ComplianceViolation {
                    control,
                    severity,
                    timestamp,
                }
            }
            Self::ComplianceScoreUpdated { score } => {
                ServiceEvent::ComplianceScoreUpdated { score, timestamp }
            }
            Self::CnappSecurityViolation {
                violation_type,
                detail,
            } => ServiceEvent::CnappSecurityViolation {
                violation_type,
                detail,
                timestamp,
            },
            Self::CopilotQueryExecuted { response } => {
                ServiceEvent::CopilotQueryExecuted { response, timestamp }
            }
            Self::InvestigationCompleted { report } => {
                ServiceEvent::InvestigationCompleted { report, timestamp }
            }
            Self::ThreatCorrelated { threat } => {
                ServiceEvent::ThreatCorrelated { threat, timestamp }
            }
            Self::AiRiskScoreUpdated { score } => {
                ServiceEvent::AiRiskScoreUpdated { score, timestamp }
            }
            Self::AiRecommendationGenerated { recommendation } => {
                ServiceEvent::AiRecommendationGenerated { recommendation, timestamp }
            }
            Self::ExecutiveReportGenerated { report } => {
                ServiceEvent::ExecutiveReportGenerated { report, timestamp }
            }
            Self::AiSecurityViolation {
                violation_type,
                detail,
            } => ServiceEvent::AiSecurityViolation {
                violation_type,
                detail,
                timestamp,
            },
            Self::PromptBlocked {
                tenant_id,
                reason,
            } => ServiceEvent::PromptBlocked {
                tenant_id,
                reason,
                timestamp,
            },
            Self::ProviderAccessDenied {
                tenant_id,
                provider,
            } => ServiceEvent::ProviderAccessDenied {
                tenant_id,
                provider,
                timestamp,
            },
        }
    }
}

/// Deprecated alias — use `ServiceEvent`.
pub type WsEvent = ServiceEvent;
