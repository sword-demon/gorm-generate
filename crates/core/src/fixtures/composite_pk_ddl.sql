CREATE TABLE `order_items` (
  `order_id` BIGINT NOT NULL,
  `item_id` BIGINT NOT NULL,
  `qty` INT NOT NULL,
  PRIMARY KEY (`order_id`, `item_id`)
) COMMENT='Order line items';
