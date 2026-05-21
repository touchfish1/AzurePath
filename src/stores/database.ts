import { defineStore } from "pinia";
import { ref } from "vue";
import {
  remoteShellListDbConnections,
  remoteShellCreateDbConnection,
  remoteShellDeleteDbConnection,
  remoteShellTestDbConnection,
  remoteShellMysqlListDatabases,
  remoteShellMysqlListTables,
  remoteShellMysqlDescribeTable,
  remoteShellMysqlExecuteQuery,
  remoteShellPgListDatabases,
  remoteShellPgListTables,
  remoteShellPgExecuteQuery,
  remoteShellRedisListKeys,
  remoteShellRedisGetValue,
  remoteShellRedisSetValue,
  remoteShellRedisSetTtl,
  type DbConnection,
  type DbConnectionInput,
  type MySqlColumnInfo,
  type MySqlQueryResult,
  type RedisKeyEntry,
} from "@/lib/tauri";
import { useToastStore } from "@/stores/toast";

export const useDatabaseStore = defineStore("database", () => {
  const connections = ref<DbConnection[]>([]);
  const activeConnection = ref<DbConnection | null>(null);
  const queryResult = ref<MySqlQueryResult | null>(null);
  const isLoading = ref(false);

  async function loadConnections(dbType?: string) {
    isLoading.value = true;
    try {
      connections.value = await remoteShellListDbConnections(dbType);
    } catch (e) {
      useToastStore().error(`加载数据库连接失败: ${e}`);
    } finally {
      isLoading.value = false;
    }
  }

  async function createConnection(input: DbConnectionInput, password: string) {
    try {
      const conn = await remoteShellCreateDbConnection(input, password);
      connections.value.push(conn);
      useToastStore().success("数据库连接已创建");
      return conn;
    } catch (e) {
      useToastStore().error(`创建数据库连接失败: ${e}`);
      throw e;
    }
  }

  async function deleteConnection(id: string) {
    try {
      await remoteShellDeleteDbConnection(id);
      connections.value = connections.value.filter((c) => c.id !== id);
      if (activeConnection.value?.id === id) {
        activeConnection.value = null;
      }
      useToastStore().success("数据库连接已删除");
    } catch (e) {
      useToastStore().error(`删除数据库连接失败: ${e}`);
      throw e;
    }
  }

  async function testConnection(id: string): Promise<string> {
    try {
      const result = await remoteShellTestDbConnection(id);
      useToastStore().success("连接测试成功");
      return result;
    } catch (e) {
      useToastStore().error(`连接测试失败: ${e}`);
      throw e;
    }
  }

  async function mysqlQuery(connId: string, database: string, query: string) {
    isLoading.value = true;
    try {
      queryResult.value = await remoteShellMysqlExecuteQuery(connId, database, query);
    } catch (e) {
      useToastStore().error(`MySQL 查询失败: ${e}`);
      throw e;
    } finally {
      isLoading.value = false;
    }
  }

  async function pgQuery(connId: string, database: string, query: string) {
    isLoading.value = true;
    try {
      queryResult.value = await remoteShellPgExecuteQuery(connId, database, query);
    } catch (e) {
      useToastStore().error(`PostgreSQL 查询失败: ${e}`);
      throw e;
    } finally {
      isLoading.value = false;
    }
  }

  async function redisListKeys(connId: string, pattern?: string): Promise<RedisKeyEntry[]> {
    try {
      return await remoteShellRedisListKeys(connId, pattern);
    } catch (e) {
      useToastStore().error(`Redis 获取键列表失败: ${e}`);
      return [];
    }
  }

  async function redisGetValue(connId: string, key: string): Promise<string> {
    try {
      return await remoteShellRedisGetValue(connId, key);
    } catch (e) {
      useToastStore().error(`Redis 获取值失败: ${e}`);
      throw e;
    }
  }

  async function redisSetValue(connId: string, key: string, value: string) {
    try {
      await remoteShellRedisSetValue(connId, key, value);
      useToastStore().success("Redis 值已设置");
    } catch (e) {
      useToastStore().error(`Redis 设置值失败: ${e}`);
      throw e;
    }
  }

  async function redisSetTtl(connId: string, key: string, ttl: number) {
    try {
      await remoteShellRedisSetTtl(connId, key, ttl);
      useToastStore().success("Redis TTL 已设置");
    } catch (e) {
      useToastStore().error(`Redis 设置 TTL 失败: ${e}`);
      throw e;
    }
  }

  async function mysqlListDatabases(connId: string): Promise<string[]> {
    try {
      return await remoteShellMysqlListDatabases(connId);
    } catch (e) {
      useToastStore().error(`获取 MySQL 数据库列表失败: ${e}`);
      return [];
    }
  }

  async function mysqlListTables(connId: string, database: string): Promise<string[]> {
    try {
      return await remoteShellMysqlListTables(connId, database);
    } catch (e) {
      useToastStore().error(`获取 MySQL 表列表失败: ${e}`);
      return [];
    }
  }

  async function mysqlDescribeTable(connId: string, database: string, table: string): Promise<MySqlColumnInfo[]> {
    try {
      return await remoteShellMysqlDescribeTable(connId, database, table);
    } catch (e) {
      useToastStore().error(`获取 MySQL 表结构失败: ${e}`);
      return [];
    }
  }

  async function pgListDatabases(connId: string): Promise<string[]> {
    try {
      return await remoteShellPgListDatabases(connId);
    } catch (e) {
      useToastStore().error(`获取 PostgreSQL 数据库列表失败: ${e}`);
      return [];
    }
  }

  async function pgListTables(connId: string, database: string): Promise<string[]> {
    try {
      return await remoteShellPgListTables(connId, database);
    } catch (e) {
      useToastStore().error(`获取 PostgreSQL 表列表失败: ${e}`);
      return [];
    }
  }

  return {
    connections,
    activeConnection,
    queryResult,
    isLoading,
    loadConnections,
    createConnection,
    deleteConnection,
    testConnection,
    mysqlQuery,
    pgQuery,
    redisListKeys,
    redisGetValue,
    redisSetValue,
    redisSetTtl,
    mysqlListDatabases,
    mysqlListTables,
    mysqlDescribeTable,
    pgListDatabases,
    pgListTables,
  };
});
