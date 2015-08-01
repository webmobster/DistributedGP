#![feature(scoped)] 
#![feature(alloc)] 
#![deny(warnings)]

#[macro_use]
extern crate log;
extern crate alloc;
extern crate distrgp_generator;
mod pool;

use std::sync::{Arc, Mutex};
use std::io::Read;

use distrgp_generator::Generator;
use distrgp_generator::BiChannel;
use distrgp_generator::OperatorMap;
use distrgp_generator::GlobalState;
use distrgp_generator::LocalState;
use distrgp_generator::StateIO;

use self::pool::ThreadPool;
use self::pool::GreenThreadData;

pub enum UtilMessage
{
	RequestData,
	Data(Vec<GlobalState>),
}

pub enum FitnessMessage
{
	Ready,
	PopVec(Vec<BiChannel<StateIO>>),
	PopFin,
	Finish

}
#[allow(unused_variables)]
pub fn init<T:Read>(mut generator: Generator<T>, numthreads: u32, fitcomms: BiChannel<FitnessMessage>, utilcomms: BiChannel<UtilMessage>)
{

	info!("started evaluator");
	

	assert!(numthreads > 0, "Need to set more than 1 evaluator threads");



	generator.initialize_operators();

	generator.generate_graphs();


	info!("generated graphs");
	

		
	let pool= Arc::new(Mutex::new(ThreadPool::new(12)));



	loop
	{

		let comms= generator.initialize_graphs();
		assert!(fitcomms.send(FitnessMessage::PopVec(comms)).is_ok());

		info!("waiting for fitness evaluator to be ready");
		match fitcomms.recv()
		{
			Ok(x) => {
				 	match x
					{
						FitnessMessage::Ready => (),
						_ => panic!("Invalid Message")
					}
				},
			_ => panic!("Dropped receiver")
		}

		

		// run all populations and send fitnesses 
		//try and get rid of the map clones
		debug!("Fix Me: multiple uneeded clones of operator map, as the borrow check cant confirm all the jobs using the map will be finished before the next loop");
		let opmap =generator.get_operator_map();

		info!("Started Evaluation");
		iterate_over_entity(generator.get_graph_list_mutref(),opmap.clone(),pool.clone());

		match fitcomms.recv()
		{
			Ok(x) => {
				 	match x
					{
						FitnessMessage::PopFin => (),
						_ => panic!("Invalid Message")
					}
				},
			_ => panic!("Dropped receiver")
		}

		info!("Evaluated Generation");

		match utilcomms.try_recv()
		{
			Ok(x) => {
				 	match x
					{
						UtilMessage::RequestData => {
											assert!(utilcomms.send(UtilMessage::Data(generator.get_graph_list_safecopy())).is_ok());
										},
						_ => panic!("Invalid Message")
					}
				},
			_ => ()
		}




		generator.reproduce();
		assert!(fitcomms.send(FitnessMessage::Finish).is_ok());

	}

			




}


fn iterate_over_entity(pop: &mut Vec<GlobalState>, map: OperatorMap, pool: Arc<Mutex<ThreadPool>>)
{


	for i in 0 .. pop.len()
	{
		let working_graph = (&mut **pop).get_mut(i).unwrap();

		let initial_local_state = LocalState::new();


		let green= GreenThreadData::new(working_graph.clone(),initial_local_state,map.clone());

		{
			let thread = working_graph.thread_count.clone().unwrap();
			let mut threadlock = thread.lock().unwrap();
			assert!(*threadlock == 0);
	
			*threadlock =1;	    
		}
		pool.lock().unwrap().execute(green,pool.clone());
	}



}	




