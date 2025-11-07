#pragma once

namespace accparser {

class accparserBaseListener {
public:
    virtual ~accparserBaseListener() = default;
};

class Async_clauseContext;
class Atomic_directiveContext;
class Attach_clauseContext;
class Auto_clauseContext;
class Bind_clauseContext;
class C_prefixContext;
class Cache_directiveContext;
class Capture_clauseContext;
class Collapse_clauseContext;
class ConditionContext;
class Const_intContext;
class Copy_clauseContext;
class Copyin_clauseContext;
class Copyin_clause_modifierContext;
class Copyin_no_modifier_clauseContext;
class Copyout_clauseContext;
class Copyout_clause_modifierContext;
class Copyout_no_modifier_clauseContext;
class Create_clauseContext;
class Create_clause_modifierContext;
class Create_no_modifier_clauseContext;
class Data_directiveContext;
class Declare_directiveContext;
class Default_async_clauseContext;
class Default_clauseContext;
class Default_kindContext;
class Delete_clauseContext;
class Detach_clauseContext;
class Device_clauseContext;
class Device_num_clauseContext;
class Device_resident_clauseContext;
class Device_type_clauseContext;
class Deviceptr_clauseContext;
class End_directiveContext;
class End_host_data_directiveContext;
class Enter_data_directiveContext;
class Exit_data_directiveContext;
class Finalize_clauseContext;
class Firstprivate_clauseContext;
class Fortran_paired_directiveContext;
class Fortran_prefixContext;
class Gang_clauseContext;
class Gang_no_list_clauseContext;
class Host_clauseContext;
class Host_data_directiveContext;
class If_clauseContext;
class If_present_clauseContext;
class Independent_clauseContext;
class Init_directiveContext;
class Int_exprContext;
class Kernels_directiveContext;
class Kernels_loop_directiveContext;
class Link_clauseContext;
class Loop_directiveContext;
class NameContext;
class Name_or_stringContext;
class No_create_clauseContext;
class Nohost_clauseContext;
class Num_gangs_clauseContext;
class Num_workers_clauseContext;
class Parallel_directiveContext;
class Parallel_loop_directiveContext;
class Present_clauseContext;
class Private_clauseContext;
class Read_clauseContext;
class Reduction_clauseContext;
class Reduction_operatorContext;
class Routine_directiveContext;
class Self_clauseContext;
class Self_list_clauseContext;
class Seq_clauseContext;
class Serial_directiveContext;
class Serial_loop_directiveContext;
class Set_directiveContext;
class Shutdown_directiveContext;
class Tile_clauseContext;
class Update_clauseContext;
class Update_directiveContext;
class Use_device_clauseContext;
class VarContext;
class Vector_clauseContext;
class Vector_clause_modifierContext;
class Vector_length_clauseContext;
class Vector_no_modifier_clauseContext;
class Wait_argument_clauseContext;
class Wait_argument_int_exprContext;
class Wait_argument_queuesContext;
class Wait_clauseContext;
class Wait_directiveContext;
class Wait_int_exprContext;
class Worker_clauseContext;
class Worker_clause_modifierContext;
class Worker_no_modifier_clauseContext;
class Write_clauseContext;

namespace accparser {
class Cache_directive_modifierContext;
}

} // namespace accparser
