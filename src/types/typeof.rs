pub struct TypeofHandler {
    // Type resolution
    type_resolver: TypeResolver,
    
    // Expression evaluation
    expr_evaluator: ExpressionEvaluator,
    
    // Type cache
    type_cache: HashMap<ExpressionId, Type>,
    
    // Validation
    validator: TypeofValidator,
}

impl TypeofHandler {
    pub fn resolve_typeof(
        &mut self,
        expr: &Expression
    ) -> Result<Type, TypeError> {
        // Check cache first
        if let Some(cached_type) = self.type_cache.get(&expr.id) {
            return Ok(cached_type.clone());
        }
        
        // Evaluate expression if needed
        let evaluated_expr = self.expr_evaluator.evaluate(expr)?;
        
        // Resolve type
        let resolved_type = self.type_resolver.resolve_type(&evaluated_expr)?;
        
        // Validate resolved type
        self.validator.validate_type(&resolved_type)?;
        
        // Cache the result
        self.type_cache.insert(expr.id, resolved_type.clone());
        
        Ok(resolved_type)
    }

    fn handle_typeof_typeof(&mut self, expr: &Expression) -> Result<Type, TypeError> {
        // Special case: typeof(typeof(x))
        // C23 specifies this should be handled specially
        self.resolve_nested_typeof(expr)
    }
} 
