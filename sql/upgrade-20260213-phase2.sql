-- ============================================================
-- TSLink IoT Core — Phase2 schema additions
-- SPEC-IOT-PHASE2 (T02)
-- ============================================================

SET NAMES utf8mb4;
SET CHARACTER SET utf8mb4;

-- 1) 产品表补充 product_secret
ALTER TABLE `product`
  ADD COLUMN IF NOT EXISTS `product_secret` VARCHAR(128) DEFAULT NULL COMMENT '产品密钥';

-- 2) 设备类型表（最小可用）
CREATE TABLE IF NOT EXISTS `device_type` (
  `id` BIGINT NOT NULL AUTO_INCREMENT,
  `code` VARCHAR(64) NOT NULL COMMENT '设备类型代码',
  `name` VARCHAR(128) NOT NULL COMMENT '设备类型名称',
  `description` VARCHAR(512) DEFAULT NULL,
  `gmt_create` DATETIME DEFAULT CURRENT_TIMESTAMP,
  `gmt_modified` DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_device_type_code` (`code`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='设备类型表';

-- 3) 设备配置表（配置下发）
CREATE TABLE IF NOT EXISTS `device_config` (
  `id` BIGINT NOT NULL AUTO_INCREMENT,
  `product_key` VARCHAR(64) NOT NULL COMMENT '产品标识',
  `device_id` VARCHAR(128) NOT NULL COMMENT '设备标识',
  `version` BIGINT DEFAULT 1 COMMENT '配置版本号',
  `config_json` JSON DEFAULT NULL COMMENT '设备配置内容',
  `gmt_create` DATETIME DEFAULT CURRENT_TIMESTAMP,
  `gmt_modified` DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_pk_did` (`product_key`, `device_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='设备配置表';
