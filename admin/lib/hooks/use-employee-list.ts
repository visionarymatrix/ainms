"use client";

import { useState, useEffect, useCallback } from "react";
import { listEmployees } from "@/lib/api/employees";
import type { Employee } from "@/lib/api/employees";
import { getCompanyId } from "@/lib/auth/session";

export function useEmployeeList() {
  const [employees, setEmployees] = useState<Employee[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchEmployees = useCallback(async () => {
    const companyId = getCompanyId();
    if (!companyId) {
      setLoading(false);
      return;
    }
    try {
      setLoading(true);
      setError(null);
      const data = await listEmployees(companyId);
      setEmployees(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch employees");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchEmployees();
  }, [fetchEmployees]);

  return { employees, loading, error, refetch: fetchEmployees };
}