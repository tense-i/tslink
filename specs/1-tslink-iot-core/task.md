
---

## 3. `specs/1-tslink-iot-core/tasks.md`

```markdown
# Tasks: TSLink IoT Core

**Input**: Design documents from `specs/1-tslink-iot-core/`
**Prerequisites**: plan.md (required), spec.md (required)

**Tests**: 集成测试在 Phase 9 统一执行，单元测试随各模块编写。

**Organization**: Tasks grouped by user story for independent implementation.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: 项目初始化、Cargo 配置、基本目录结构

- [ ] T001 初始化 Cargo.toml，配置 workspace 元数据和依赖：tokio, rumqttc, axum, serde, serde_json, redis, rdkafka, sqlx, tracing, tracing-subscriber, prometheus, config, thiserror, dashmap, uuid, chrono
- [ ] T002 [P] 创建目录结构：src/{domain,application,infrastructure/{mqtt,redis,kafka,database,http}}, tests/{common,integration,bench}, config/
- [ ] T003 [P] 创建配置文件 config/default.toml 和 config/dev.toml，定义 MQTT/Redis/Kafka/MySQL/HTTP 配置项
- [ ] T004 [P] 实现配置加载模块 src/config.rs —— 定义 AppConfig / MqttConfig / RedisConfig / KafkaConfig / DatabaseConfig / HttpConfig 结构体，支持 TOML + 环境变量覆盖
- [ ] T005 [P] 实现全局错误类型 src/error.rs —— 定义 TsLinkError 枚举（MqttError, RedisError, KafkaError, DatabaseError, ConfigError, TopicParseError, SerializeError, TimeoutError），实现 thiserror derive
- [ ] T006 [P] 实现 telemetry 初始化 src/telemetry.rs —— tracing-subscriber 初始化（JSON 格式）、prometheus Registry 创建、全局 metrics 定义（mqtt_messages_total, mqtt_message_latency_seconds, device_online_count, redis_operation_duration_seconds）
- [ ] T007 创建 src/main.rs 骨架 —— 加载配置 → 初始化 telemetry → 占位符启动 MQTT/HTTP（后续 US 填充）

**Checkpoint**: `cargo build` 和 `cargo clippy` 通过，配置可加载，日志可输出

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 领域层核心类型定义，所有 User Story 共享

- [ ] T008 [P] 定义设备实体 src/domain/device.rs —— Device struct（product_key, device_id, device_name, device_secret, device_status, parent_product_key, parent_id, gmt_last_online, register_time, device_extend, org_code）、DeviceStatus enum（Online, Offline, Fault, NotActive）
- [ ] T009 [P] 定义消息类型 src/domain/message.rs —— CommonTopicReceiver<T> struct（tid, bid, version, timestamp, method, product_key, device_id, data: T, code, message）、CommonTopicResponse<T> struct、serde Serialize/Deserialize derive、实现 new/reply 构建方法
- [ ] T010 [P] 定义 Topic 类型 src/domain/topic.rs —— TopicInfo struct（product_key, device_id, link_suffix, category, sub_category, identifier, level）、MessageType enum（Thing, Platform, App）、ThingMessageType enum（EventProperty, EventCustom, ServiceReply, PropertySetReply, Register, DynamicRegister, Pong, Ntp, UpdateTopo, DeviceModel, DeviceRequest）、PlatformMessageType enum、AppMessageType enum
- [ ] T011 [P] 定义物模型类型 src/domain/thing_model.rs —— DeviceModel struct（name, product_key, device_id, product_version, product_type, properties, services, events, configs）、FunctionMethod struct（identifier, method, call_type, input_fields, output_fields）、MethodField struct（name, identifier, data_type）、CallType enum（Sync, Async）
- [ ] T012 [P] 定义影子类型 src/domain/shadow.rs —— DeviceShadow struct（product_key, device_id, properties: serde_json::Value, updated_at）、ShadowServiceConfig struct（product_key, method, payload）
- [ ] T013 [P] 定义链路类型 src/domain/link.rs —— Link struct（link_id, product_key, device_id, is_active, weight, last_message_time）
- [ ] T014 [P] 定义领域事件 src/domain/event.rs —— DomainEvent enum（DeviceOnline{pk,did}, DeviceOffline{pk,did}, PropertyChanged{pk,did,properties}, ServiceInvoked{pk,did,method}, EventReceived{pk,did,identifier,level,data}）
- [ ] T015 创建领域层 mod.rs src/domain/mod.rs —— 导出所有子模块

**Checkpoint**: `cargo build` 通过，所有领域类型可编译，serde 序列化/反序列化单元测试通过

---

## Phase 3: User Story 1 - MQTT 连接与消息收发 (Priority: P0) 🎯 MVP

**Goal**: TSLink 能连接 EMQX、订阅 topic、接收/发布消息

**Independent Test**: 启动 TSLink 连接到 EMQX，发送 MQTT 消息，验证接收和反序列化

### Implementation for User Story 1

- [ ] T016 [US1] 实现 MQTT 客户端封装 src/infrastructure/mqtt/client.rs —— MqttClient struct 封装 rumqttc AsyncClient + EventLoop，实现 connect / subscribe / publish / reconnect 方法，支持配置化 MqttOptions（host/port/credentials/keepalive/cleanSession）
- [ ] T017 [US1] 实现 Topic 解析器 src/infrastructure/mqtt/topic_parser.rs —— parse_topic(topic: &str) → Result<TopicInfo> 函数，从 `sys/{pk}/{did}/...` 和 `region/{region}/sys/{pk}/{did}/...` 中提取 product_key / device_id / link_suffix / category / sub_category / identifier，处理级联前缀剥离
- [ ] T018 [US1] 实现消息发布器 src/infrastructure/mqtt/publisher.rs —— MessagePublisher struct，实现 publish(topic, response) / publish_with_reply(topic, response, timeout) 方法，PostSync 使用 DashMap<String, tokio::sync::oneshot::Sender> 按 tid 注册等待者，eventloop 收到 reply 时通过 tid 匹配唤醒
- [ ] T019 [US1] 实现 MQTT EventLoop 处理 —— 在 client.rs 中 spawn_event_loop 方法，循环 poll EventLoop，将 Incoming::Publish 事件反序列化为 CommonTopicReceiver 并通过 tokio mpsc channel 送入路由引擎
- [ ] T020 [US1] 在 src/main.rs 中集成 MQTT 客户端启动 —— 读取 MqttConfig → 创建 MqttClient → 连接 → 订阅 inbound topics → spawn eventloop → 消息 channel 传递到路由层

**Checkpoint**: TSLink 启动并连接 EMQX，能接收消息并打印日志，能发布消息

---

## Phase 4: User Story 2 - Topic 路由与消息分发 (Priority: P0)

**Goal**: 按 topic 模式将消息分发到正确的 handler

**Independent Test**: 发送不同 topic 的 MQTT 消息，验证每条消息到达对应 handler

### Implementation for User Story 2

- [ ] T021 [US2] 定义 MessageHandler trait src/infrastructure/mqtt/handler.rs —— `trait MessageHandler: Send + Sync { async fn handle(&self, topic: &TopicInfo, msg: CommonTopicReceiver<serde_json::Value>) -> Result<()>; fn message_type(&self) -> &[ThingMessageType]; }`
- [ ] T022 [US2] 实现路由引擎 src/infrastructure/mqtt/router.rs —— MessageRouter struct，持有 HashMap<ThingMessageType, Arc<dyn MessageHandler>>，实现 route(topic_str, payload) 方法：parse_topic → match MessageType → lookup handler → dispatch，支持级联消息前缀剥离，未匹配 topic 记录 warn
- [ ] T023 [US2] 注册路由 —— 在 main.rs 或独立 bootstrap 模块中，将各 handler 注册到 MessageRouter（此阶段先注册 NoopHandler 占位），将 router 与 MQTT eventloop channel 对接
- [ ] T024 [US2] 为 TopicInfo 和路由引擎编写单元测试 —— 覆盖所有 17 个 inbound topic 模式的解析和路由匹配

**Checkpoint**: 消息从 MQTT → 解析 → 路由 → handler 全链路通畅，日志可追踪

---

## Phase 5: User Story 3 - 设备生命周期管理 (Priority: P0)

**Goal**: 设备上线/离线状态管理、动态注册

**Independent Test**: 模拟设备连接/断线，验证 Redis 状态更新

### Implementation for User Story 5

- [ ] T025 [P] [US3] 实现 Redis 设备状态存储 src/infrastructure/redis/device_state.rs —— DeviceStateRedis struct，实现 set_online(pk, did) / set_offline(pk, did) / get_status(pk, did) / refresh_heartbeat(pk, did)，Redis key 格式 `DEVICE_STATUS_{pk}_{did}`
- [ ] T026 [P] [US3] 实现设备数据库仓库 src/infrastructure/database/device_repo.rs —— DeviceRepository struct (sqlx::MySqlPool)，实现 find_by_pk_did / create / update_status / find_by_product_key / verify_secret 方法
- [ ] T027 [US3] 实现设备应用服务 src/application/device_service.rs —— DeviceService struct，实现 handle_device_online(pk, did) / handle_device_offline(pk, did) / handle_heartbeat(pk, did) / handle_register(pk, did, secret, receiver) / handle_dynamic_register(pk, did, sn, receiver)，编排 Redis + DB 操作
- [ ] T028 [US3] 实现 SYS Topic handler —— 处理 `$SYS/brokers/+/clients/{clientId}/connected` 和 `disconnected`，从 clientId 解析 deviceId，调用 DeviceService
- [ ] T029 [US3] 实现心跳 handler —— 处理 `sys/+/+/thing/pong/post`，调用 DeviceService.handle_heartbeat
- [ ] T030 [US3] 实现注册 handler —— 处理 `sys/+/+/thing/register/post` 和 `dynamic_register/post`，调用 DeviceService.handle_register，回复注册结果到 `register/post_reply`
- [ ] T031 [US3] 实现 NTP 处理 src/application/ntp_service.rs —— 处理 `sys/+/+/thing/ntp/post`，回复服务器时间戳到 `ntp/post_reply`

**Checkpoint**: 设备连接 EMQX 后 Redis 状态更新为 ONLINE，断线更新为 OFFLINE，动态注册可创建设备

---

## Phase 6: User Story 4 - 设备影子 (Priority: P1)

**Goal**: Redis 属性影子读写 + 上线影子重放

**Independent Test**: 写入影子后查询，设备上线后自动重放

### Implementation for User Story 4

- [ ] T032 [P] [US4] 实现 Redis 影子存储 src/infrastructure/redis/shadow.rs —— ShadowRedis struct，实现 get_properties(pk, did) → serde_json::Value / merge_properties(pk, did, new_props) / delete(pk, did)，Redis key `IOT_DEVICE_PROPERTIES_KEY_{pk}_{did}`
- [ ] T033 [P] [US4] 实现影子配置数据库仓库 src/infrastructure/database/shadow_repo.rs —— ShadowRepository struct，实现 find_shadow_services(pk) → Vec<ShadowServiceConfig> / upsert_shadow_service(pk, did, method, payload)
- [ ] T034 [US4] 实现影子应用服务 src/application/shadow_service.rs —— ShadowService struct，实现 get_device_properties(pk, did) / update_properties(pk, did, props) / update_service(pk, did, method, data) / replay_shadow_on_online(pk, did) —— 上线重放读取 ShadowServiceConfig 并逐个发送服务调用
- [ ] T035 [US4] 实现属性上报 handler —— 处理 `sys/+/+/thing/event/property/post`，解析属性数据，调用 ShadowService.update_properties

**Checkpoint**: 设备上报属性后 Redis 影子更新，设备上线时影子服务自动重放

---

## Phase 7: User Story 5 - 物模型服务调用 (Priority: P1)

**Goal**: 平台向设备发起同步/异步服务调用

**Independent Test**: 通过代码调用 service invoke，验证 MQTT 消息发布和回复匹配

### Implementation for User Story 5

- [ ] T036 [P] [US5] 实现物模型加载 src/infrastructure/database/model_repo.rs —— ModelRepository struct，实现 get_device_model(pk, did) → DeviceModel，从 product/module/function/function_param 表加载，使用 moka cache 缓存 30 秒
- [ ] T037 [US5] 实现物模型服务调用 src/application/thing_service.rs —— ThingService struct，实现 invoke_service(pk, did, method, data, sync: bool) → Result<CommonTopicReceiver>，构建 CommonTopicResponse → 通过 MessagePublisher 发布到 `sys/{pk}/{did}/thing/service/{method}/post`，同步时使用 publish_with_reply 等待回复
- [ ] T038 [US5] 实现属性设置 —— 在 ThingService 中实现 set_properties(pk, did, properties)，发布到 `sys/{pk}/{did}/thing/properties/set`
- [ ] T039 [US5] 实现服务回复 handler —— 处理 `sys/+/+/thing/service/+/post_reply`，通过 MessagePublisher 的 PostSync 按 tid 匹配唤醒等待者
- [ ] T040 [US5] 在服务调用成功后更新影子 —— ThingService.invoke_service 成功后调用 ShadowService.update_service 记录调用

**Checkpoint**: 服务调用 → MQTT 发布 → 设备回复 → 同步返回，完整链路通畅

---

## Phase 8: User Story 6 - 物模型事件处理 (Priority: P1)

**Goal**: 接收设备事件并转发到 Kafka

**Independent Test**: 模拟设备上报事件，验证 ACK 回复和 Kafka 消息

### Implementation for User Story 6

- [ ] T041 [P] [US6] 实现 Kafka 生产者 src/infrastructure/kafka/producer.rs —— EventProducer struct 封装 rdkafka FutureProducer，实现 send_event(topic, key, payload) 方法，错误时 fallback 到日志
- [ ] T042 [US6] 实现事件应用服务 src/application/event_service.rs —— EventService struct，实现 handle_event(pk, did, identifier, level, receiver)：回复 ACK（`{level}_reply`）→ 发送到 Kafka `iot-device-message` topic
- [ ] T043 [US6] 实现事件 handler —— 处理 `sys/+/+/thing/event/{identifier}/{info|warning|error}`，调用 EventService.handle_event

**Checkpoint**: 设备上报事件 → ACK 回复 → Kafka 消息可消费

---

## Phase 9: User Story 7 - 多链路设备通信 (Priority: P2)

**Goal**: 多链路权重计算、主链路选择、路由策略

**Independent Test**: 模拟双链路设备，验证权重计算和链路选择

### Implementation for User Story 7

- [ ] T044 [P] [US7] 实现 Redis 链路存储 src/infrastructure/redis/link.rs —— LinkRedis struct，实现 Lua 脚本滑动窗口 QPS 计算、update_link_weight(pk, did, link_id) / get_links(pk, did) → Vec<Link> / get_active_link(pk, did) / set_active_link(pk, did, link_id)，Redis key `DEVICE_LINK_{pk}_{linkId}` (ZSet), `DEVICE_MULTILINK_{pk}_{did}` (ZSet), `ACTIVE_DEVICE_LINK_{pk}_{did}` (String)
- [ ] T045 [US7] 实现链路应用服务 src/application/link_service.rs —— LinkService struct，实现 check_link(pk, did, msg) → Option<String> (从 clientId 尾部提取 linkSuffix，计算权重) / select_link(pk, did) → String (手动优先 → 最高权重) / get_links(pk, did) / set_active_link(pk, did, link_id)
- [ ] T046 [US7] 在 Topic 解析中支持 linkSuffix —— topic_parser.rs 中从 deviceId 段解析 `{did}_{linkSuffix}` 格式
- [ ] T047 [US7] 在 DeviceService/ThingService 中集成多链路 —— 服务调用时通过 LinkService.select_link 确定目标 topic 的 deviceId 段

**Checkpoint**: 多链路设备消息正确路由，主链路自动切换

---

## Phase 10: User Story 8 - HTTP 管理 API (Priority: P2)

**Goal**: RESTful API 提供设备查询、服务调用、影子管理

**Independent Test**: curl 调用 API 验证响应

### Implementation for User Story 8

- [ ] T048 [P] [US8] 实现 HTTP Server 启动 src/infrastructure/http/server.rs —— 配置 axum Router，绑定端口，集成 tracing middleware
- [ ] T049 [P] [US8] 实现路由定义 src/infrastructure/http/routes.rs —— GET /api/v1/devices/{pk}/{did}, POST /api/v1/devices/{pk}/{did}/services/{method}, GET /api/v1/devices/{pk}/{did}/shadow, PUT /api/v1/devices/{pk}/{did}/shadow, GET /api/v1/devices/{pk}/{did}/links, PUT /api/v1/devices/{pk}/{did}/links/active, GET /health, GET /metrics
- [ ] T050 [P] [US8] 实现设备 API handler src/infrastructure/http/device_handler.rs —— get_device / list_devices
- [ ] T051 [P] [US8] 实现服务调用 API handler src/infrastructure/http/service_handler.rs —— invoke_service（调用 ThingService）
- [ ] T052 [P] [US8] 实现影子 API handler src/infrastructure/http/shadow_handler.rs —— get_shadow / update_shadow
- [ ] T053 [US8] 实现健康检查 src/infrastructure/http/health.rs —— /health 返回组件状态（mqtt/redis/kafka/db），/metrics 返回 prometheus 格式指标
- [ ] T054 [US8] 在 src/main.rs 中集成 HTTP server 启动 —— 与 MQTT eventloop 并行运行

**Checkpoint**: 所有 HTTP API 可通过 curl 调用，返回正确 JSON 响应

---

## Phase 11: Polish & Cross-Cutting Concerns

**Purpose**: 文档、Docker、部署、性能验证

- [ ] T055 [P] 创建 Dockerfile —— 多阶段构建（builder 阶段 cargo build --release → 运行阶段 debian-slim），暴露 HTTP 端口和 metrics 端口
- [ ] T056 [P] 创建 .env.example —— 列出所有环境变量及默认值
- [ ] T057 [P] 创建 deploy/ Helm chart 骨架 —— Chart.yaml + values.yaml + templates/deployment.yaml + templates/service.yaml
- [ ] T058 [P] 编写 Topic 路由性能基准测试 tests/bench/routing_bench.rs —— 验证 10K topic/s 路由分发性能
- [ ] T059 [P] 编写消息序列化基准测试 tests/bench/message_bench.rs —— 验证 CommonTopicReceiver 反序列化 50K msg/s
- [ ] T060 编写集成测试 tests/integration/ —— 使用 testcontainers 启动 EMQX + Redis，验证完整 MQTT 消息 → 路由 → 设备状态 → 影子更新 → 服务调用 链路
- [ ] T061 代码审查：cargo clippy --all-targets -- -D warnings，cargo fmt --check
- [ ] T062 编写 README.md —— 项目介绍、架构图、快速启动、配置说明、API 文档

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: 无依赖，立即开始
- **Foundational (Phase 2)**: 依赖 Setup 完成
- **US1 MQTT (Phase 3)**: 依赖 Foundational
- **US2 路由 (Phase 4)**: 依赖 US1
- **US3 设备管理 (Phase 5)**: 依赖 US2
- **US4 影子 (Phase 6)**: 依赖 US3（设备上线事件触发影子重放）
- **US5 服务调用 (Phase 7)**: 依赖 US1（MessagePublisher）+ US4（影子更新）
- **US6 事件处理 (Phase 8)**: 依赖 US2（路由）
- **US7 多链路 (Phase 9)**: 依赖 US5（服务调用集成）
- **US8 HTTP API (Phase 10)**: 依赖 US3/US4/US5（调用各 Service）
- **Polish (Phase 11)**: 依赖所有 US 完成

### Within Each User Story

- Infrastructure adapters (Redis/DB) 可并行开发 [P]
- Application service 依赖 infrastructure adapters
- Handler 注册依赖 application service
- main.rs 集成在最后

### Parallel Opportunities

- Phase 2 所有 domain 类型定义 (T008-T015) 全部可并行
- Phase 5 Redis + DB adapter (T025, T026) 可并行
- Phase 6 Redis + DB adapter (T032, T033) 可并行
- Phase 10 所有 HTTP handler (T048-T052) 可并行
- Phase 11 所有文档/部署 (T055-T059) 可并行

---

## Implementation Strategy

### MVP First (US1 + US2 + US3)

1. Phase 1: Setup → 项目可编译
2. Phase 2: Domain types → 类型系统建立
3. Phase 3: MQTT 连接 → 能收发消息
4. Phase 4: Topic 路由 → 消息可分发
5. Phase 5: 设备管理 → 设备上下线可感知
6. **STOP**: 验证 MVP — TSLink 能连接 EMQX、接收消息、感知设备上下线

### Incremental Delivery

7. Phase 6: 影子 → 设备属性可缓存和查询
8. Phase 7: 服务调用 → 可控制设备
9. Phase 8: 事件处理 → 事件可转发到 Kafka
10. Phase 9: 多链路 → 支持多链路设备
11. Phase 10: HTTP API → 外部系统可集成
12. Phase 11: 部署/文档 → 上线就绪

---

## Notes

- [P] tasks = 不同文件，无依赖，可并行
- [Story] label 映射到 spec.md 中的 User Story
- 每个 Phase 完成后执行 `cargo build && cargo clippy && cargo test`
- 每完成一个 checkpoint 提交一次 git commit
- 总计 62 个 task，预估 ~40 人天（2人约 4 周）