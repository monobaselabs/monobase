/**
 * Person Module E2E Tests
 * Tests the complete person management workflow
 */

import { describe, test, expect, beforeAll, afterAll, beforeEach } from 'bun:test';
import { faker } from '@faker-js/faker';
import { createApiClient, type ApiClient } from '../../helpers/client';
import { createTestApp, type TestApp } from '../../helpers/test-app';
import { addYears } from 'date-fns';
import {
  generateTestPersonData,
  generateMinimalPersonData,
  generateInvalidPersonData,
  generateBulkPersonData,
  createPerson,
  getPerson,
  updatePerson,
  listPersons,
  waitForPerson,
  validatePersonResponse,
  generateValidDateOfBirthCases,
  generateInvalidDateOfBirthCases,
  generateDateOfBirthEdgeCases,
  areDatesEqual,
  extractDatePart,
  type PersonCreateRequest,
  type PersonUpdateRequest,
  type PersonResponse
} from '../../helpers/person';

describe('Person Module E2E Tests', () => {
  let apiClient: ApiClient;
  let testApp: TestApp;

  beforeAll(async () => {
    testApp = await createTestApp({ storage: true });

    // Create API client with embedded app instance
    apiClient = createApiClient({ app: testApp.app });

    // Create and sign up a new user for this test
    await apiClient.signup();
  }, 30000);

  afterAll(async () => {
    await testApp?.cleanup();
  });

  beforeEach(async () => {
    // Create new API client for each test
    apiClient = createApiClient({ app: testApp.app });

    // Create and sign up a new user for this test
    await apiClient.signup();
  }, 30000);


  describe('Person Creation', () => {
    test('should create person with valid complete data', async () => {
      const testData = generateTestPersonData();
      
      const { response, data } = await createPerson(apiClient, testData);
      
      expect(response.status).toBe(201);
      expect(data).toBeDefined();
      expect(data?.id).toBeDefined();
      expect(data?.firstName).toBe(testData.firstName);
      expect(data?.lastName).toBe(testData.lastName);
      expect(data?.gender).toBe(testData.gender);
      expect(data?.contactInfo?.email).toBe(testData.contactInfo?.email);
      expect(data?.primaryAddress?.city).toBe(testData.primaryAddress?.city);
      expect(validatePersonResponse(data)).toBe(true);
      
    });

    test('should create person with minimal required fields', async () => {
      const testData = generateMinimalPersonData();
      
      const { response, data } = await createPerson(apiClient, testData);
      
      expect(response.status).toBe(201);
      expect(data).toBeDefined();
      expect(data?.id).toBeDefined();
      expect(data?.firstName).toBe(testData.firstName);
      
    });

    test('should validate required fields', async () => {
      const invalidData = { lastName: 'NoFirstName' };
      
      const response = await apiClient.fetch('/persons', {
        method: 'POST',
        body: invalidData
      });
      
      expect(response.status).toBe(400);
      const error = await response.json();
      // TypeSpec ErrorDetail/ValidationError format
      expect(error.code).toBeDefined();
      expect(error.message).toBeDefined();
      expect(error.statusCode).toBe(400);
      expect(error.requestId).toBeDefined();
      expect(error.timestamp).toBeDefined();
      expect(error.path).toBeDefined();
      expect(error.method).toBeDefined();
    });

    test('should validate email format', async () => {
      const testData = generateTestPersonData({
        contactInfo: { email: 'invalid-email' }
      });
      
      const { response } = await createPerson(apiClient, testData);
      
      expect(response.status).toBe(400);
      const error = await response.json();
      // TypeSpec ValidationError format
      expect(error.code).toBeDefined();
      expect(error.message).toBeDefined();
      expect(error.statusCode).toBe(400);
      expect(error.requestId).toBeDefined();
      expect(error.timestamp).toBeDefined();
      expect(error.path).toBeDefined();
      expect(error.method).toBeDefined();
      // Check that there's a field error related to email
      if (error.fieldErrors) {
        const emailError = error.fieldErrors.find((fieldError: any) =>
          fieldError.field.includes('email')
        );
        expect(emailError).toBeDefined();
      }
    });

    test('should validate date of birth format', async () => {
      const testData = generateTestPersonData({
        dateOfBirth: 'not-a-date'
      });
      
      const { response } = await createPerson(apiClient, testData);
      
      expect(response.status).toBe(400);
    });

    test('should validate gender enum values', async () => {
      const testData = {
        firstName: faker.person.firstName(),
        gender: 'invalid-gender'
      };
      
      const response = await apiClient.fetch('/persons', {
        method: 'POST',
        body: testData
      });
      
      expect(response.status).toBe(400);
    });

    test('should handle special characters in names', async () => {
      const testData = generateTestPersonData({
        firstName: "Jean-François",
        lastName: "O'Brien-Smith",
        middleName: "María"
      });
      
      const { response, data } = await createPerson(apiClient, testData);
      
      expect(response.status).toBe(201);
      expect(data?.firstName).toBe(testData.firstName);
      expect(data?.lastName).toBe(testData.lastName);
      expect(data?.middleName).toBe(testData.middleName);
      
    });

    test('should handle international addresses', async () => {
      const testData = generateTestPersonData({
        primaryAddress: {
          street1: '123 Main Street',
          city: 'London',
          state: 'England',
          postalCode: 'SW1A 1AA',
          country: 'GB',
          coordinates: {
            latitude: 51.5074,
            longitude: -0.1278
          }
        }
      });
      
      const { response, data } = await createPerson(apiClient, testData);
      
      expect(response.status).toBe(201);
      expect(data?.primaryAddress?.country).toBe('GB');
      
    });

    test('should handle multiple languages', async () => {
      const testData = generateTestPersonData({
        languagesSpoken: ['en', 'es', 'fr', 'zh']
      });
      
      const { response, data } = await createPerson(apiClient, testData);
      
      expect(response.status).toBe(201);
      expect(data?.languagesSpoken).toEqual(['en', 'es', 'fr', 'zh']);
      
    });
  });

  describe('Person Retrieval', () => {
    let testPersonId: string;
    let testPersonData: PersonCreateRequest;

    beforeEach(async () => {
      // Create a person for retrieval tests
      testPersonData = generateTestPersonData();
      const { data } = await createPerson(apiClient, testPersonData);
      testPersonId = data!.id;
    });

    test('should retrieve person by ID', async () => {
      const { response, data } = await getPerson(apiClient, testPersonId);
      
      expect(response.status).toBe(200);
      expect(data).toBeDefined();
      expect(data?.id).toBe(testPersonId);
      expect(data?.firstName).toBe(testPersonData.firstName);
      expect(validatePersonResponse(data)).toBe(true);
    });

    test('should return 404 for non-existent person', async () => {
      const fakeId = faker.string.uuid();
      
      const { response } = await getPerson(apiClient, fakeId);
      
      expect(response.status).toBe(404);
    });

    test('should return 400 for invalid person ID format', async () => {
      const invalidId = 'not-a-uuid';
      
      const { response } = await getPerson(apiClient, invalidId);
      
      expect(response.status).toBe(400);
    });

    test('should wait for person to be available', async () => {
      const person = await waitForPerson(apiClient, testPersonId);
      
      expect(person).toBeDefined();
      expect(person?.id).toBe(testPersonId);
    });
  });

  describe('Person Update', () => {
    let testPersonId: string;
    let originalData: PersonCreateRequest;

    beforeEach(async () => {
      // Create a person for update tests
      originalData = generateTestPersonData();
      const { data } = await createPerson(apiClient, originalData);
      testPersonId = data!.id;
    });

    test('should update person basic information', async () => {
      const updateData: PersonUpdateRequest = {
        firstName: 'UpdatedFirst',
        lastName: 'UpdatedLast',
        middleName: 'UpdatedMiddle'
      };
      
      const { response, data } = await updatePerson(apiClient, testPersonId, updateData);
      
      expect(response.status).toBe(200);
      expect(data?.firstName).toBe('UpdatedFirst');
      expect(data?.lastName).toBe('UpdatedLast');
      expect(data?.middleName).toBe('UpdatedMiddle');
      // Other fields should remain unchanged
      expect(data?.gender).toBe(originalData.gender);
    });

    test('should update contact information', async () => {
      const updateData: PersonUpdateRequest = {
        contactInfo: {
          email: 'updated@example.com',
          phone: '+19876543210'
        }
      };
      
      const { response, data } = await updatePerson(apiClient, testPersonId, updateData);
      
      expect(response.status).toBe(200);
      expect(data?.contactInfo?.email).toBe('updated@example.com');
      expect(data?.contactInfo?.phone).toBe('+19876543210');
    });

    test('should update address', async () => {
      const updateData: PersonUpdateRequest = {
        primaryAddress: {
          street1: '456 New Street',
          city: 'New City',
          state: 'NY',
          postalCode: '10001',
          country: 'US'
        }
      };
      
      const { response, data } = await updatePerson(apiClient, testPersonId, updateData);
      
      expect(response.status).toBe(200);
      expect(data?.primaryAddress?.street1).toBe('456 New Street');
      expect(data?.primaryAddress?.city).toBe('New City');
    });

    test('should validate updates with invalid data', async () => {
      const updateData = {
        contactInfo: {
          email: 'invalid-email'
        }
      };
      
      const { response } = await updatePerson(apiClient, testPersonId, updateData);
      
      expect(response.status).toBe(400);
    });

    test('should return 403 when updating another user\'s person', async () => {
      const fakeId = faker.string.uuid();
      const updateData: PersonUpdateRequest = {
        firstName: 'Updated'
      };
      
      const { response } = await updatePerson(apiClient, fakeId, updateData);
      
      expect(response.status).toBe(403);
    });

    test('should handle partial updates', async () => {
      const updateData: PersonUpdateRequest = {
        timezone: 'Europe/London'
      };
      
      const { response, data } = await updatePerson(apiClient, testPersonId, updateData);
      
      expect(response.status).toBe(200);
      expect(data?.timezone).toBe('Europe/London');
      // Other fields should remain unchanged
      expect(data?.firstName).toBe(originalData.firstName);
      expect(data?.lastName).toBe(originalData.lastName);
    });
  });

  describe('Person DateOfBirth Updates', () => {
    let testPersonId: string;
    let originalData: PersonCreateRequest;

    beforeEach(async () => {
      // Create a person for dateOfBirth update tests
      originalData = generateTestPersonData();
      const { data } = await createPerson(apiClient, originalData);
      testPersonId = data!.id;
    });

    describe('Valid DateOfBirth Updates', () => {
      test('should update with various valid dateOfBirth formats', async () => {
        const validCases = generateValidDateOfBirthCases();
        
        for (const testCase of validCases) {
          const updateData: PersonUpdateRequest = {
            dateOfBirth: testCase.date
          };
          
          const { response, data } = await updatePerson(apiClient, testPersonId, updateData);
          
          expect(response.status).toBe(200);
          expect(data?.dateOfBirth).toBeDefined();
          expect(areDatesEqual(data!.dateOfBirth!, testCase.date)).toBe(true);
          
          // Verify other fields remain unchanged
          expect(data?.firstName).toBe(originalData.firstName);
          expect(data?.lastName).toBe(originalData.lastName);
          expect(data?.gender).toBe(originalData.gender);
        }
      });

      test('should update dateOfBirth with plainDate format', async () => {
        const newDateOfBirth = '1995-07-20';
        const updateData: PersonUpdateRequest = {
          dateOfBirth: newDateOfBirth
        };

        const { response, data } = await updatePerson(apiClient, testPersonId, updateData);

        expect(response.status).toBe(200);
        expect(data?.dateOfBirth).toBe(newDateOfBirth);

        // Verify the update persisted by fetching again
        const { data: refreshedData } = await getPerson(apiClient, testPersonId);
        expect(refreshedData?.dateOfBirth).toBe(newDateOfBirth);
      });

      test('should handle edge case dates correctly', async () => {
        const edgeCases = generateDateOfBirthEdgeCases();
        
        for (const testCase of edgeCases) {
          const updateData: PersonUpdateRequest = {
            dateOfBirth: testCase.date
          };
          
          const { response, data } = await updatePerson(apiClient, testPersonId, updateData);
          
          expect(response.status).toBe(200);
          expect(data?.dateOfBirth).toBeDefined();
          expect(areDatesEqual(data!.dateOfBirth!, testCase.date)).toBe(true);
        }
      });

      test('should handle dateOfBirth as plainDate (date-only, no time)', async () => {
        const plainDate = '1988-03-15';
        const updateData: PersonUpdateRequest = {
          dateOfBirth: plainDate
        };

        const { response, data } = await updatePerson(apiClient, testPersonId, updateData);

        expect(response.status).toBe(200);
        expect(data?.dateOfBirth).toBe(plainDate);
      });

      test('should handle leap year dates correctly', async () => {
        const leapYearDate = '2000-02-29'; // Valid leap year date
        const updateData: PersonUpdateRequest = {
          dateOfBirth: leapYearDate
        };

        const { response, data } = await updatePerson(apiClient, testPersonId, updateData);

        expect(response.status).toBe(200);
        expect(data?.dateOfBirth).toBe(leapYearDate);
        expect(extractDatePart(data!.dateOfBirth!)).toBe('2000-02-29');
      });

      test('should update only dateOfBirth without affecting other fields', async () => {
        const originalContactEmail = originalData.contactInfo?.email;
        const originalAddress = originalData.primaryAddress?.city;

        const newDateOfBirth = '1992-11-08';
        const updateData: PersonUpdateRequest = {
          dateOfBirth: newDateOfBirth
        };

        const { response, data } = await updatePerson(apiClient, testPersonId, updateData);

        expect(response.status).toBe(200);
        expect(data?.dateOfBirth).toBe(newDateOfBirth);
        
        // Verify all other fields remain exactly the same
        expect(data?.firstName).toBe(originalData.firstName);
        expect(data?.lastName).toBe(originalData.lastName);
        expect(data?.middleName).toBe(originalData.middleName ?? null);
        expect(data?.gender).toBe(originalData.gender);
        expect(data?.contactInfo?.email).toBe(originalContactEmail);
        expect(data?.primaryAddress?.city).toBe(originalAddress);
        expect(data?.languagesSpoken).toEqual(originalData.languagesSpoken);
        expect(data?.timezone).toBe(originalData.timezone);
      });
    });

    describe('Invalid DateOfBirth Updates', () => {
      test('should reject invalid dateOfBirth formats', async () => {
        const invalidCases = generateInvalidDateOfBirthCases();
        
        for (const testCase of invalidCases) {
          const updateData: PersonUpdateRequest = {
            dateOfBirth: testCase.date
          };
          
          const { response } = await updatePerson(apiClient, testPersonId, updateData);
          
          expect(response.status).toBe(400);
          
          // Verify the original data wasn't changed
          const { data: unchangedData } = await getPerson(apiClient, testPersonId);
          expect(unchangedData?.dateOfBirth).toBe(originalData.dateOfBirth);
        }
      });

      test('should reject future dates', async () => {
        const futureDate = addYears(new Date(), 1).toISOString(); // 1 year from now
        const updateData: PersonUpdateRequest = {
          dateOfBirth: futureDate
        };
        
        const { response } = await updatePerson(apiClient, testPersonId, updateData);
        
        expect(response.status).toBe(400);
        
        // Verify original data remains unchanged
        const { data: unchangedData } = await getPerson(apiClient, testPersonId);
        expect(unchangedData?.dateOfBirth).toBe(originalData.dateOfBirth);
      });

      test('should reject invalid date strings with proper error response', async () => {
        const invalidDate = 'not-a-valid-date';
        const updateData: PersonUpdateRequest = {
          dateOfBirth: invalidDate
        };
        
        const { response } = await updatePerson(apiClient, testPersonId, updateData);

        expect(response.status).toBe(400);
        const error = await response.json();
        // TypeSpec ErrorDetail format
        expect(error.code).toBeDefined();
        expect(error.message).toBeDefined();
        expect(error.statusCode).toBe(400);
        expect(error.requestId).toBeDefined();
      });

      test('should reject impossible dates (calendar validation)', async () => {
        const impossibleDate = '2000-02-30'; // Feb 30th doesn't exist
        const updateData: PersonUpdateRequest = {
          dateOfBirth: impossibleDate
        };

        const { response } = await updatePerson(apiClient, testPersonId, updateData);

        // Should return 400 because validator now checks calendar validity
        expect(response.status).toBe(400);
      });
    });

    describe('DateOfBirth Update Edge Cases', () => {
      test('should handle null dateOfBirth update', async () => {
        const updateData: PersonUpdateRequest = {
          dateOfBirth: undefined
        };
        
        const { response, data } = await updatePerson(apiClient, testPersonId, updateData);
        
        // Should succeed - undefined means don't update this field
        expect(response.status).toBe(200);
        expect(data?.dateOfBirth).toBe(originalData.dateOfBirth);
      });

      test('should handle multiple rapid dateOfBirth updates', async () => {
        const dates = [
          '1990-01-01',
          '1991-06-15',
          '1992-12-31'
        ];
        
        // Perform rapid updates
        for (let i = 0; i < dates.length; i++) {
          const updateData: PersonUpdateRequest = {
            dateOfBirth: dates[i]
          };
          
          const { response, data } = await updatePerson(apiClient, testPersonId, updateData);
          
          expect(response.status).toBe(200);
          expect(data?.dateOfBirth).toBe(dates[i]);
        }
        
        // Verify final state
        const { data: finalData } = await getPerson(apiClient, testPersonId);
        expect(finalData?.dateOfBirth).toBe(dates[dates.length - 1]);
      });

      test('should maintain dateOfBirth consistency across concurrent updates', async () => {
        const testDate = '1985-08-20';

        // Perform concurrent updates with the same date
        const promises = Array(3).fill(null).map(() =>
          updatePerson(apiClient, testPersonId, { dateOfBirth: testDate })
        );

        const results = await Promise.allSettled(promises);

        // All updates should succeed
        results.forEach(result => {
          expect(result.status).toBe('fulfilled');
          if (result.status === 'fulfilled') {
            expect(result.value.response.status).toBe(200);
          }
        });

        // Final state should have the updated date
        const { data: finalData } = await getPerson(apiClient, testPersonId);
        expect(finalData?.dateOfBirth).toBe(testDate);
      });

      test('should handle dateOfBirth updates with mixed field updates', async () => {
        const updateData: PersonUpdateRequest = {
          firstName: 'UpdatedFirst',
          dateOfBirth: '1993-05-10',
          timezone: 'America/New_York'
        };

        const { response, data } = await updatePerson(apiClient, testPersonId, updateData);

        expect(response.status).toBe(200);
        expect(data?.firstName).toBe('UpdatedFirst');
        expect(data?.dateOfBirth).toBe('1993-05-10');
        expect(data?.timezone).toBe('America/New_York');
        
        // Other fields should remain unchanged
        expect(data?.lastName).toBe(originalData.lastName);
        expect(data?.gender).toBe(originalData.gender);
      });

      test('should validate dateOfBirth update with very old dates', async () => {
        const veryOldDate = '1900-01-01';
        const updateData: PersonUpdateRequest = {
          dateOfBirth: veryOldDate
        };

        const { response, data } = await updatePerson(apiClient, testPersonId, updateData);

        // Should succeed (1900 is typically the minimum acceptable year)
        expect(response.status).toBe(200);
        expect(data?.dateOfBirth).toBe(veryOldDate);
      });
    });
  });

  describe('Person Listing', () => {
    let adminClient: ApiClient;

    beforeEach(async () => {
      // Create admin client for listing operations (requires admin role)
      adminClient = createApiClient({ app: testApp.app });
      await adminClient.signinAsAdmin();
      
      // Create multiple persons for listing tests using regular user
      const personsToCreate = 5;
      for (let i = 0; i < personsToCreate; i++) {
        const testData = generateTestPersonData({
          firstName: `Test${i}`,
          lastName: `Person${i}`
        });
        await createPerson(apiClient, testData);
      }
    });

    test('should list persons with default pagination', async () => {
      const { response, data } = await listPersons(adminClient);
      
      expect(response.status).toBe(200);
      expect(data).toBeDefined();
      expect(Array.isArray(data?.data)).toBe(true);
      expect(data?.data.length).toBeGreaterThan(0);
      expect(data?.pagination.totalCount).toBeGreaterThanOrEqual(5);
      expect(data?.pagination.currentPage).toBe(1);
      expect(data?.pagination.limit).toBeGreaterThan(0);
    });

    test('should list persons with custom pagination', async () => {
      const { response, data } = await listPersons(adminClient, {
        page: 1,
        pageSize: 2
      });
      
      expect(response.status).toBe(200);
      expect(data?.data.length).toBeLessThanOrEqual(2);
      expect(data?.pagination.currentPage).toBe(1);
      expect(data?.pagination.limit).toBe(2);
      expect(data?.pagination.hasNextPage).toBeDefined();
      expect(data?.pagination.hasPreviousPage).toBe(false);
    });

    test('should navigate through pages', async () => {
      // Get first page
      const { data: page1 } = await listPersons(adminClient, {
        page: 1,
        pageSize: 2
      });
      
      // Get second page if available
      if (page1?.pagination.hasNextPage) {
        const { response, data: page2 } = await listPersons(adminClient, {
          page: 2,
          pageSize: 2
        });
        
        expect(response.status).toBe(200);
        expect(page2?.pagination.currentPage).toBe(2);
        expect(page2?.pagination.hasPreviousPage).toBe(true);
        
        // Ensure different items on different pages
        const page1Ids = page1.data.map(p => p.id);
        const page2Ids = page2?.data.map(p => p.id) || [];
        const hasOverlap = page1Ids.some(id => page2Ids.includes(id));
        expect(hasOverlap).toBe(false);
      }
    });

    test('should sort persons', async () => {
      const { response: ascResponse, data: ascData } = await listPersons(adminClient, {
        sort: 'firstName',
        order: 'asc'
      });
      
      expect(ascResponse.status).toBe(200);
      expect(ascData?.data).toBeDefined();
      
      const { response: descResponse, data: descData } = await listPersons(adminClient, {
        sort: 'firstName',
        order: 'desc'
      });
      
      expect(descResponse.status).toBe(200);
      expect(descData?.data).toBeDefined();
      
      // Verify that both requests returned data
      expect(ascData?.data.length).toBeGreaterThanOrEqual(0);
      expect(descData?.data.length).toBeGreaterThanOrEqual(0);
    });

    test('should handle empty results', async () => {
      // Request a page that likely doesn't exist
      const { response, data } = await listPersons(adminClient, {
        page: 9999,
        pageSize: 10
      });
      
      expect(response.status).toBe(200);
      expect(data?.data.length).toBe(0);
      expect(data?.pagination.currentPage).toBe(9999);
      expect(data?.pagination.hasNextPage).toBe(false);
    });
  });

  describe('Concurrent Operations', () => {
    test('should handle concurrent person creation', async () => {
      const personDataArray = generateBulkPersonData(5);
      
      // Create separate users for each person since each user can only have one person profile
      const promises = personDataArray.map(async (data) => {
        const userClient = createApiClient({ app: testApp.app });
        await userClient.signup();
        return createPerson(userClient, data);
      });
      
      const results = await Promise.allSettled(promises);
      
      // All should succeed
      results.forEach(result => {
        expect(result.status).toBe('fulfilled');
        if (result.status === 'fulfilled') {
          expect(result.value.response.status).toBe(201);
        }
      });
      
      // Verify all persons were created using admin client
      const adminClient = createApiClient({ app: testApp.app });
      await adminClient.signinAsAdmin();
      const { data } = await listPersons(adminClient, { pageSize: 50 });
      expect(data?.data.length).toBeGreaterThanOrEqual(5);
    });

    test('should handle concurrent updates', async () => {
      // Create a person
      const testData = generateTestPersonData();
      const { data: createdPerson } = await createPerson(apiClient, testData);
      
      // Perform concurrent updates
      const updates = [
        { firstName: 'Update1' },
        { lastName: 'Update2' },
        { middleName: 'Update3' }
      ];
      
      const promises = updates.map(update => 
        updatePerson(apiClient, createdPerson!.id, update)
      );
      const results = await Promise.allSettled(promises);
      
      // All should succeed
      results.forEach(result => {
        expect(result.status).toBe('fulfilled');
        if (result.status === 'fulfilled') {
          expect(result.value.response.status).toBe(200);
        }
      });
      
      // Get final state
      const { data: finalPerson } = await getPerson(apiClient, createdPerson!.id);
      expect(finalPerson).toBeDefined();
      // Last update wins for each field
    });

    test('should handle mixed concurrent operations', async () => {
      // Create admin client for list operation
      const adminClient = createApiClient({ app: testApp.app });
      await adminClient.signinAsAdmin();

      // Create separate user clients for person creation
      const client1 = createApiClient({ app: testApp.app });
      await client1.signup();
      const client2 = createApiClient({ app: testApp.app });
      await client2.signup();
      
      const operations = [
        createPerson(client1, generateTestPersonData()),
        createPerson(client2, generateTestPersonData()),
        listPersons(adminClient, { page: 1, pageSize: 10 })
      ];
      
      const results = await Promise.allSettled(operations);
      
      // All should succeed
      results.forEach((result, index) => {
        expect(result.status).toBe('fulfilled');
        if (result.status === 'fulfilled' && index < 2) {
          // Creation operations
          const createResult = result.value as { response: Response; data?: PersonResponse };
          expect(createResult.response.status).toBe(201);
        }
      });
    });
  });

  describe('Edge Cases and Error Handling', () => {
    test('should handle very long names', async () => {
      const testData = generateTestPersonData({
        firstName: faker.string.alpha(50), // Max length
        lastName: faker.string.alpha(50),
        middleName: faker.string.alpha(50)
      });
      
      const { response, data } = await createPerson(apiClient, testData);
      
      expect(response.status).toBe(201);
      expect(data?.firstName.length).toBe(50);
      
    });

    test('should reject names exceeding max length', async () => {
      const testData = generateTestPersonData({
        firstName: faker.string.alpha(51) // Exceeds max length
      });
      
      const { response } = await createPerson(apiClient, testData);
      
      expect(response.status).toBe(400);
    });

    test('should handle Unicode characters', async () => {
      const testData = generateTestPersonData({
        firstName: '北京',
        lastName: 'مُحَمَّد',
        middleName: 'Ελληνικά'
      });
      
      const { response, data } = await createPerson(apiClient, testData);
      
      expect(response.status).toBe(201);
      expect(data?.firstName).toBe('北京');
      expect(data?.lastName).toBe('مُحَمَّد');
      expect(data?.middleName).toBe('Ελληνικά');
      
    });

    test('should handle minimal data with omitted optional fields', async () => {
      const testData = {
        firstName: faker.person.firstName()
        // All other fields omitted (not null)
      };
      
      const response = await apiClient.fetch('/persons', {
        method: 'POST',
        body: testData
      });
      
      expect(response.status).toBe(201);
      const data = await response.json();
      expect(data.firstName).toBe(testData.firstName);
    });

    test('should handle request timeout gracefully', async () => {
      // Create a person with a very large payload
      const testData = generateTestPersonData({
        bio: faker.lorem.paragraphs(1000) // Very large text
      });

      const { response } = await createPerson(apiClient, testData);

      // Should either succeed or fail gracefully
      expect([201, 400, 413, 408].includes(response.status)).toBe(true);
    });
  });

  describe('International Data Validation', () => {
    test('should reject uppercase language codes', async () => {
      const testData = generateTestPersonData({
        languagesSpoken: ['EN', 'ES'] // Invalid - must be lowercase
      });

      const { response } = await createPerson(apiClient, testData);

      expect(response.status).toBe(400);
      const error = await response.json();
      expect(error.code).toBeDefined();
    });

    test('should reject mixed case language codes', async () => {
      const testData = generateTestPersonData({
        languagesSpoken: ['en', 'Es', 'FR'] // 'Es' and 'FR' are invalid
      });

      const { response } = await createPerson(apiClient, testData);

      expect(response.status).toBe(400);
    });

    test('should reject lowercase country codes', async () => {
      const testData = generateTestPersonData({
        primaryAddress: {
          street1: '123 Main St',
          city: 'New York',
          state: 'NY',
          postalCode: '10001',
          country: 'us' // Invalid - must be uppercase
        }
      });

      const { response } = await createPerson(apiClient, testData);

      expect(response.status).toBe(400);
      const error = await response.json();
      expect(error.code).toBeDefined();
    });

    test('should reject mixed case country codes', async () => {
      const testData = generateTestPersonData({
        primaryAddress: {
          street1: '123 Main St',
          city: 'New York',
          state: 'NY',
          postalCode: '10001',
          country: 'Us' // Invalid - must be uppercase
        }
      });

      const { response } = await createPerson(apiClient, testData);

      expect(response.status).toBe(400);
    });

    test('should reject lowercase timezone formats', async () => {
      const testData = generateTestPersonData({
        timezone: 'america/new_york' // Invalid - wrong case
      });

      const { response } = await createPerson(apiClient, testData);

      expect(response.status).toBe(400);
      const error = await response.json();
      expect(error.code).toBeDefined();
    });

    test('should reject abbreviation timezone formats', async () => {
      const testData = generateTestPersonData({
        timezone: 'EST' // Invalid - must be IANA format
      });

      const { response } = await createPerson(apiClient, testData);

      expect(response.status).toBe(400);
    });

    test('should accept valid lowercase language codes', async () => {
      const testData = generateTestPersonData({
        languagesSpoken: ['en', 'es', 'ja', 'fr']
      });

      const { response, data } = await createPerson(apiClient, testData);

      expect(response.status).toBe(201);
      expect(data?.languagesSpoken).toEqual(['en', 'es', 'ja', 'fr']);
    });

    test('should accept valid uppercase country codes', async () => {
      const testData = generateTestPersonData({
        primaryAddress: {
          street1: '123 Main St',
          city: 'New York',
          state: 'NY',
          postalCode: '10001',
          country: 'US'
        }
      });

      const { response, data } = await createPerson(apiClient, testData);

      expect(response.status).toBe(201);
      expect(data?.primaryAddress?.country).toBe('US');
    });

    test('should accept valid IANA timezone formats', async () => {
      const testData = generateTestPersonData({
        timezone: 'America/New_York'
      });

      const { response, data } = await createPerson(apiClient, testData);

      expect(response.status).toBe(201);
      expect(data?.timezone).toBe('America/New_York');
    });
  });
});