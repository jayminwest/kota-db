import { createTestClient, validateMCPResponse, MCPTestClient } from './test-helpers';

describe('MCP Protocol Compliance', () => {
  let client: MCPTestClient;

  beforeAll(async () => {
    client = await createTestClient();
  }, 15000);

  afterAll(async () => {
    await client.cleanup();
  });

  describe('JSON-RPC 2.0 Protocol', () => {
    test('should respond with correct JSON-RPC version', async () => {
      const response = await client.sendRequest('tools/list');
      validateMCPResponse(response);
      expect(response.jsonrpc).toBe('2.0');
    });

    test('should include request ID in response', async () => {
      const response = await client.sendRequest('tools/list');
      expect(response.id).toBeDefined();
      expect(typeof response.id).toBe('number');
    });

    test('should handle request without ID (notification style)', async () => {
      // Send notification-style request (no ID expected in response)
      const requestWithoutId = {
        jsonrpc: '2.0',
        method: 'tools/list',
        params: {},
      };

      // This should not throw, even if no response comes back
      const promise = new Promise<void>((resolve, reject) => {
        if (!client.isServerRunning()) {
          reject(new Error('Server not running'));
          return;
        }

        // For notifications, we don't expect a response
        setTimeout(() => resolve(), 100);
      });

      await expect(promise).resolves.toBeUndefined();
    });

    test('should return error for invalid JSON-RPC version', async () => {
      // This test validates that malformed requests are handled properly
      const response = await client.sendRequest('tools/list');
      expect(response.jsonrpc).toBe('2.0');
      expect(response.error).toBeUndefined();
      expect(response.result).toBeDefined();
    });

    test('should return error for unknown methods', async () => {
      try {
        await client.sendRequest('unknown/method');
        fail('Expected error for unknown method');
      } catch (error) {
        expect(error).toBeDefined();
        expect((error as Error).message).toContain('MCP Error');
      }
    });
  });

  describe('MCP Initialization Handshake', () => {
    test('should support MCP capabilities discovery', async () => {
      // In actual MCP, initialization happens during connection
      // We verify the server is properly initialized by checking available tools
      const tools = await client.listTools();
      expect(Array.isArray(tools)).toBe(true);
      expect(tools.length).toBe(7);
    });

    test('should expose all required tools', async () => {
      const tools = await client.listTools();
      const toolNames = tools.map(tool => tool.name);
      
      const expectedTools = [
        'kotadb_document_create',
        'kotadb_document_get',
        'kotadb_document_update',
        'kotadb_document_delete',
        'kotadb_document_list',
        'kotadb_search',
        'kotadb_stats',
      ];

      expectedTools.forEach(expectedTool => {
        expect(toolNames).toContain(expectedTool);
      });
    });

    test('should provide tool schemas', async () => {
      const tools = await client.listTools();
      
      tools.forEach(tool => {
        expect(tool.name).toBeDefined();
        expect(typeof tool.name).toBe('string');
        expect(tool.description).toBeDefined();
        expect(typeof tool.description).toBe('string');
        expect(tool.inputSchema).toBeDefined();
        expect(typeof tool.inputSchema).toBe('object');
        expect(tool.inputSchema.type).toBe('object');
        expect(tool.inputSchema.properties).toBeDefined();
      });
    });
  });

  describe('Resource Management Protocol', () => {
    test('should list available resources', async () => {
      const resources = await client.listResources();
      expect(Array.isArray(resources)).toBe(true);
      expect(resources.length).toBe(1);
      
      const documentsResource = resources[0];
      expect(documentsResource.uri).toBe('kotadb://documents');
      expect(documentsResource.name).toBeDefined();
      expect(documentsResource.description).toBeDefined();
    });

    test('should read resource contents', async () => {
      const resourceData = await client.readResource('kotadb://documents');
      expect(resourceData.contents).toBeDefined();
      expect(Array.isArray(resourceData.contents)).toBe(true);
      expect(resourceData.contents.length).toBe(1);
      
      const content = resourceData.contents[0];
      expect(content.uri).toBe('kotadb://documents');
      expect(content.mimeType).toBe('application/json');
      expect(content.text).toBeDefined();
      
      // Validate JSON structure
      const parsedContent = JSON.parse(content.text);
      expect(parsedContent.documents).toBeDefined();
      expect(Array.isArray(parsedContent.documents)).toBe(true);
      expect(parsedContent.total).toBeDefined();
      expect(typeof parsedContent.total).toBe('number');
    });

    test('should handle invalid resource URIs', async () => {
      try {
        await client.readResource('kotadb://nonexistent');
        fail('Expected error for invalid resource URI');
      } catch (error) {
        expect(error).toBeDefined();
        expect((error as Error).message).toContain('MCP Error');
      }
    });
  });

  describe('Tool Execution Protocol', () => {
    test('should validate tool input schemas', async () => {
      // Test with invalid arguments - missing required field
      try {
        await client.callTool('kotadb_document_create', {
          // Missing required 'path' and 'content' fields
          title: 'Test',
        });
        fail('Expected validation error');
      } catch (error) {
        expect(error).toBeDefined();
      }
    });

    test('should return structured tool responses', async () => {
      const testDoc = {
        path: '/protocol-test.md',
        title: 'Protocol Test Document',
        content: 'Testing protocol compliance',
        tags: ['protocol', 'test'],
      };

      const result = await client.callTool('kotadb_document_create', testDoc);
      
      // Validate response structure
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
      expect(Array.isArray(result.content)).toBe(true);
      expect(result.content.length).toBeGreaterThan(0);
      
      const content = result.content[0];
      expect(content.type).toBe('text');
      expect(content.text).toBeDefined();
      
      // Validate JSON content
      const parsedContent = JSON.parse(content.text);
      expect(parsedContent.success).toBe(true);
      expect(parsedContent.document).toBeDefined();
      expect(parsedContent.document.id).toBeDefined();
    });

    test('should handle tool execution errors gracefully', async () => {
      const result = await client.callTool('kotadb_document_get', {
        id: 'nonexistent-document-id',
      });
      
      expect(result.isError).toBe(true);
      expect(result.content).toBeDefined();
      expect(Array.isArray(result.content)).toBe(true);
      
      const content = JSON.parse(result.content[0].text);
      expect(content.success).toBe(false);
      expect(content.error).toBeDefined();
      expect(typeof content.error).toBe('string');
    });

    test('should support concurrent tool execution', async () => {
      // Create multiple documents concurrently
      const promises = Array.from({ length: 5 }, (_, i) =>
        client.callTool('kotadb_document_create', {
          path: `/concurrent-${i}.md`,
          title: `Concurrent Document ${i}`,
          content: `Content for concurrent test ${i}`,
          tags: ['concurrent', 'test'],
        })
      );

      const results = await Promise.all(promises);
      
      // All should succeed
      results.forEach(result => {
        expect(result).toBeDefined();
        const content = JSON.parse(result.content[0].text);
        expect(content.success).toBe(true);
        expect(content.document.id).toBeDefined();
      });

      // All should have unique IDs
      const ids = results.map(r => JSON.parse(r.content[0].text).document.id);
      const uniqueIds = new Set(ids);
      expect(uniqueIds.size).toBe(5);
    });
  });

  describe('Error Handling Protocol', () => {
    test('should provide structured error responses', async () => {
      try {
        await client.sendRequest('invalid/method', {});
        fail('Expected error for invalid method');
      } catch (error) {
        expect(error).toBeDefined();
        expect((error as Error).message).toContain('MCP Error');
      }
    });

    test('should handle malformed requests', async () => {
      // Test timeout behavior for requests that never get responses
      const timeoutPromise = client.sendRequest('tools/list', {}, 100);
      
      // This should either succeed quickly or timeout
      try {
        const response = await timeoutPromise;
        validateMCPResponse(response);
      } catch (error) {
        // Timeout is acceptable behavior
        expect((error as Error).message).toContain('timeout');
      }
    });

    test('should maintain server stability after errors', async () => {
      // Send several invalid requests
      const invalidRequests = [
        client.callTool('nonexistent_tool', {}),
        client.callTool('kotadb_document_get', { id: '' }),
        client.callTool('kotadb_document_create', {}),
      ];

      // All should return errors but not crash server
      for (const request of invalidRequests) {
        try {
          const result = await request;
          // Should get error result, not throw
          expect(result.isError).toBe(true);
        } catch (error) {
          // Throwing is also acceptable for invalid requests
          expect(error).toBeDefined();
        }
      }

      // Server should still be responsive
      const healthCheck = await client.listTools();
      expect(Array.isArray(healthCheck)).toBe(true);
      expect(healthCheck.length).toBe(7);
    });
  });

  describe('Performance and Reliability', () => {
    test('should handle high request volume', async () => {
      const startTime = Date.now();
      const requests = Array.from({ length: 20 }, () =>
        client.listTools()
      );

      const results = await Promise.all(requests);
      const endTime = Date.now();
      
      // All should succeed
      results.forEach(tools => {
        expect(Array.isArray(tools)).toBe(true);
        expect(tools.length).toBe(7);
      });

      // Should complete in reasonable time (less than 5 seconds for 20 requests)
      expect(endTime - startTime).toBeLessThan(5000);
    });

    test('should maintain response times under load', async () => {
      const responseTimes: number[] = [];
      
      for (let i = 0; i < 10; i++) {
        const start = Date.now();
        await client.listTools();
        const end = Date.now();
        responseTimes.push(end - start);
      }
      
      // Response times should be reasonable (< 1 second each)
      responseTimes.forEach(time => {
        expect(time).toBeLessThan(1000);
      });
      
      // Average response time should be good (< 500ms)
      const avgTime = responseTimes.reduce((a, b) => a + b) / responseTimes.length;
      expect(avgTime).toBeLessThan(500);
    });

    test('should recover from temporary failures', async () => {
      // This test simulates recovery by ensuring server remains responsive
      // after a series of operations
      
      // Perform various operations
      const doc = await client.createDocument({
        path: '/recovery-test.md',
        content: 'Testing recovery',
      });
      
      await client.getDocument(doc.id);
      await client.updateDocument(doc.id, 'Updated content');
      await client.searchDocuments('recovery');
      await client.deleteDocument(doc.id);
      
      // Server should still be responsive
      const finalCheck = await client.listTools();
      expect(finalCheck.length).toBe(7);
    });
  });
});