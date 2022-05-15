---
title: How to Analyze User Retention With Databend
sidebar_label: Analyzing User Retention
---

User retention helps you analyze how many users return to your product or service. Let's go through an example and see how to analyze it in Databend.

## Step 1. Databend

### 1.1 Deploy Databend

Make sure you have installed Databend, if not please see:

* [How to Deploy Databend](../00-guides/index.md#deployment)

### 1.2 Create a Table

Connect to Databend server with MySQL client:
```shell
mysql -h127.0.0.1 -uroot -P3307 
```

```sql
CREATE TABLE events(`user_id` INT, `login_date` DATE);
```

### 1.3 Prepare data

Connect to Databend server with MySQL client:
```shell
mysql -h127.0.0.1 -uroot -P3307 
```

Create a user:
```sql
INSERT INTO events SELECT number AS user_id, '2022-05-15' FROM numbers(1000000);
INSERT INTO events SELECT number AS user_id, '2022-05-16' FROM numbers(900000);
INSERT INTO events SELECT number As user_id, '2022-05-17' FROM numbers(100000);
```
## Step 2. User Retention Analysis

It's **easy** and **performance** to use [Databend Retention Function](../30-reference/20-functions/10-aggregate-functions/aggregate-retention.md) to do the user retention analysis.

```sql
SELECT
    sum(r[0]) AS r1,
    sum(r[1]) AS r2,
    sum(r[2]) AS r3
FROM
(
    SELECT
        user_id,
        retention(login_date = '2022-05-15', login_date = '2022-05-16', login_date = '2022-05-17') AS r
    FROM events
    GROUP BY user_id
);
```

The retention result is:
```sql
+---------+--------+--------+
| r1      | r2     | r3     |
+---------+--------+--------+
| 1000000 | 900000 | 100000 |
+---------+--------+--------+
```

**Enjoy your journey.** 