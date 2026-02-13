-- ============================================================
-- TSLink IoT Core — 初始化 SQL
-- 自动由 docker-compose MySQL 容器在首次启动时执行
-- ============================================================

SET NAMES utf8mb4;
SET CHARACTER SET utf8mb4;

-- ── 产品表 ────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS `product` (
  `id` BIGINT NOT NULL AUTO_INCREMENT,
  `product_key` VARCHAR(64) NOT NULL COMMENT '产品唯一标识',
  `name` VARCHAR(128) DEFAULT NULL COMMENT '产品名称',
  `product_version` VARCHAR(32) DEFAULT NULL COMMENT '产品版本',
  `product_type` VARCHAR(32) DEFAULT NULL COMMENT '产品类型 (DirectDevice/Gateway/SubDevice/UnrealDevice)',
  `description` VARCHAR(512) DEFAULT NULL,
  `gmt_create` DATETIME DEFAULT CURRENT_TIMESTAMP,
  `gmt_modified` DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_product_key` (`product_key`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='产品表';

-- ── 模块表 ────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS `module` (
  `id` BIGINT NOT NULL AUTO_INCREMENT,
  `product_id` BIGINT NOT NULL COMMENT '关联产品 ID',
  `name` VARCHAR(128) DEFAULT NULL COMMENT '模块名称',
  `identifier` VARCHAR(128) DEFAULT NULL COMMENT '模块标识',
  `description` VARCHAR(512) DEFAULT NULL,
  `gmt_create` DATETIME DEFAULT CURRENT_TIMESTAMP,
  `gmt_modified` DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  KEY `idx_product_id` (`product_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='物模型模块表';

-- ── 功能定义表 ────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS `function_info` (
  `id` BIGINT NOT NULL AUTO_INCREMENT,
  `module_id` BIGINT NOT NULL COMMENT '关联模块 ID',
  `identifier` VARCHAR(128) NOT NULL COMMENT '功能标识',
  `method` VARCHAR(256) NOT NULL COMMENT '方法名 (如 thing.service.reboot)',
  `name` VARCHAR(128) DEFAULT NULL COMMENT '功能名称',
  `call_type` VARCHAR(16) DEFAULT 'ASYNC' COMMENT '调用类型 (SYNC/ASYNC)',
  `function_type` VARCHAR(16) DEFAULT 'service' COMMENT '功能类型 (service/event/property)',
  `description` VARCHAR(512) DEFAULT NULL,
  `gmt_create` DATETIME DEFAULT CURRENT_TIMESTAMP,
  `gmt_modified` DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  KEY `idx_module_id` (`module_id`),
  KEY `idx_method` (`method`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='物模型功能定义表';

-- ── 功能参数表 ────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS `function_param` (
  `id` BIGINT NOT NULL AUTO_INCREMENT,
  `function_id` BIGINT NOT NULL COMMENT '关联功能 ID',
  `name` VARCHAR(128) NOT NULL COMMENT '参数名称',
  `identifier` VARCHAR(128) NOT NULL COMMENT '参数标识',
  `data_type` VARCHAR(32) DEFAULT NULL COMMENT '数据类型 (int/float/string/bool/struct/array)',
  `required` TINYINT(1) DEFAULT 0 COMMENT '是否必填',
  `description` VARCHAR(512) DEFAULT NULL,
  `direction` VARCHAR(16) DEFAULT 'input' COMMENT '方向 (input/output)',
  `gmt_create` DATETIME DEFAULT CURRENT_TIMESTAMP,
  `gmt_modified` DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  KEY `idx_function_id` (`function_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='功能参数表';

-- ── IoT 设备表 ────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS `iot_device` (
  `id` BIGINT NOT NULL AUTO_INCREMENT,
  `product_id` BIGINT DEFAULT NULL COMMENT '关联产品 ID',
  `product_key` VARCHAR(64) NOT NULL COMMENT '产品标识',
  `device_id` VARCHAR(128) NOT NULL COMMENT '设备标识',
  `device_name` VARCHAR(128) DEFAULT NULL COMMENT '设备名称',
  `device_secret` VARCHAR(128) DEFAULT NULL COMMENT '设备密钥',
  `device_status` VARCHAR(16) DEFAULT 'NOT_ACTIVE' COMMENT '设备状态 (ONLINE/OFFLINE/FAULT/NOT_ACTIVE)',
  `parent_product_key` VARCHAR(64) DEFAULT NULL COMMENT '父产品标识 (子设备)',
  `parent_id` VARCHAR(128) DEFAULT NULL COMMENT '父设备 ID (子设备)',
  `gmt_last_online` DATETIME DEFAULT NULL COMMENT '最后上线时间',
  `register_time` DATETIME DEFAULT NULL COMMENT '注册时间',
  `device_extend` TEXT DEFAULT NULL COMMENT '设备扩展信息 (JSON)',
  `org_code` VARCHAR(64) DEFAULT NULL COMMENT '组织编码',
  `gmt_create` DATETIME DEFAULT CURRENT_TIMESTAMP,
  `gmt_modified` DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_pk_did` (`product_key`, `device_id`),
  KEY `idx_product_key` (`product_key`),
  KEY `idx_device_status` (`device_status`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='IoT 设备表';

-- ── 设备影子服务配置表 ────────────────────────────────────────
CREATE TABLE IF NOT EXISTS `iot_device_shadow_service` (
  `id` BIGINT NOT NULL AUTO_INCREMENT,
  `product_key` VARCHAR(64) NOT NULL COMMENT '产品标识',
  `device_id` VARCHAR(128) DEFAULT NULL COMMENT '设备标识',
  `method` VARCHAR(256) NOT NULL COMMENT '服务方法名',
  `payload` TEXT DEFAULT NULL COMMENT '影子数据 (JSON)',
  `is_enabled` TINYINT(1) DEFAULT 1 COMMENT '是否启用',
  `gmt_create` DATETIME DEFAULT CURRENT_TIMESTAMP,
  `gmt_modified` DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_pk_did_method` (`product_key`, `device_id`, `method`),
  KEY `idx_product_key` (`product_key`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='设备影子服务配置表';

-- ── 示例数据 (可选, 方便开发调试) ──────────────────────────────
INSERT IGNORE INTO `product` (`product_key`, `name`, `product_version`, `product_type`)
VALUES ('demo_pk', '示例产品', '1.0', 'DirectDevice');

INSERT IGNORE INTO `module` (`product_id`, `name`, `identifier`)
SELECT id, '默认模块', 'default' FROM `product` WHERE `product_key` = 'demo_pk';

INSERT IGNORE INTO `iot_device` (`product_id`, `product_key`, `device_id`, `device_name`, `device_secret`, `device_status`, `register_time`)
SELECT id, 'demo_pk', 'demo_did_001', '示例设备', 'secret123', 'NOT_ACTIVE', NOW()
FROM `product` WHERE `product_key` = 'demo_pk';
